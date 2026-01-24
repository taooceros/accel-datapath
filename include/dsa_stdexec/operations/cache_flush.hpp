#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_CACHE_FLUSH_HPP
#define DSA_STDEXEC_OPERATIONS_CACHE_FLUSH_HPP

#include <cstring>
#include <cstdint>
#include <dsa/dsa.hpp>
#include <dsa/dsa_operation_base.hpp>
#include <dsa_stdexec/error.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <exception>
#include <stdexec/execution.hpp>
#include <utility>

namespace dsa_stdexec {

// Cache flush using Intel DSA hardware.
// Flushes CPU cache lines for the specified memory region.

template <class DsaType, class ReceiverId>
class CacheFlushOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  CacheFlushOperation(DsaType &dsa, void *dst, size_t size, Receiver r)
      : dsa_(dsa), dst_(dst), size_(size), r_(std::move(r)) {}

  CacheFlushOperation(CacheFlushOperation &&) = delete;

  void start() noexcept {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();

    memset(desc, 0, sizeof(*desc));

    desc->opcode = DSA_OPCODE_CFLUSH;
    desc->flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    desc->xfer_size = static_cast<uint32_t>(size_);
    desc->dst_addr = reinterpret_cast<uint64_t>(dst_);
    desc->completion_addr = reinterpret_cast<uint64_t>(comp);

    memset(comp, 0, sizeof(*comp));

    proxy = pro::make_proxy<OperationFacade>(Wrapper{this});

    try {
      dsa_.submit(this, desc);
    } catch (...) {
      stdexec::set_error(std::move(r_), std::current_exception());
    }
  }

private:
  struct Wrapper {
    CacheFlushOperation *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return op->get_descriptor(); }
  };

  dsa_hw_desc *get_descriptor() { return desc_ptr(); }

  void notify() {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();
    uint8_t status = comp->status & DSA_COMP_STATUS_MASK;
    if (status == DSA_COMP_SUCCESS) {
      stdexec::set_value(std::move(r_));
    } else if (status == DSA_COMP_PAGE_FAULT_NOBOF) {
      int wr = comp->status & DSA_COMP_STATUS_WRITE;
      volatile char *t = (char *)comp->fault_addr;
      wr ? *t = *t : *t;
      desc->dst_addr += comp->bytes_completed;
      desc->xfer_size -= comp->bytes_completed;
      memset(comp, 0, sizeof(*comp));
      try {
        dsa_.submit(this, desc);
      } catch (...) {
        stdexec::set_error(std::move(r_), std::current_exception());
      }
    } else {
      auto err = DsaError(status, *comp, desc->opcode, "cache_flush");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(std::move(err)));
    }
  }

private:
  DsaType &dsa_;
  void *dst_;
  size_t size_;
  Receiver r_;
};

template <class DsaType>
class CacheFlushSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  CacheFlushSender(DsaType &dsa, void *dst, size_t size)
      : dsa_(dsa), dst_(dst), size_(size) {}

  auto connect(stdexec::receiver auto &&r) {
    return CacheFlushOperation<DsaType, stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, dst_, size_, std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  void *dst_;
  size_t size_;
};

template <class DsaType>
inline CacheFlushSender<DsaType> dsa_cache_flush(DsaType &dsa, void *dst,
                                                  size_t size) {
  return CacheFlushSender<DsaType>(dsa, dst, size);
}

} // namespace dsa_stdexec

#endif
