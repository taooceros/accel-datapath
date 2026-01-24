#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_MEM_FILL_HPP
#define DSA_STDEXEC_OPERATIONS_MEM_FILL_HPP

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

template <class DsaType, class ReceiverId>
class MemFillOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  MemFillOperation(DsaType &dsa, void *dst, size_t size, uint64_t pattern, Receiver r)
      : dsa_(dsa), dst_(dst), size_(size), pattern_(pattern), r_(std::move(r)) {}

  MemFillOperation(MemFillOperation &&) = delete;

  void start() noexcept {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();

    memset(desc, 0, sizeof(*desc));

    desc->opcode = DSA_OPCODE_MEMFILL;
    desc->flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    desc->xfer_size = static_cast<uint32_t>(size_);
    desc->pattern = pattern_;
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
    MemFillOperation *op;
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
      auto err = DsaError(status, *comp, desc->opcode, "mem_fill");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(std::move(err)));
    }
  }

private:
  DsaType &dsa_;
  void *dst_;
  size_t size_;
  uint64_t pattern_;
  Receiver r_;
};

template <class DsaType>
class MemFillSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  MemFillSender(DsaType &dsa, void *dst, size_t size, uint64_t pattern)
      : dsa_(dsa), dst_(dst), size_(size), pattern_(pattern) {}

  auto connect(stdexec::receiver auto &&r) {
    return MemFillOperation<DsaType, stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, dst_, size_, pattern_, std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  void *dst_;
  size_t size_;
  uint64_t pattern_;
};

template <class DsaType>
inline MemFillSender<DsaType> dsa_mem_fill(DsaType &dsa, void *dst, size_t size,
                                            uint64_t pattern) {
  return MemFillSender<DsaType>(dsa, dst, size, pattern);
}

} // namespace dsa_stdexec

#endif
