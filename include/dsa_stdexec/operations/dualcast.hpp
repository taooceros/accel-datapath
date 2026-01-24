#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_DUALCAST_HPP
#define DSA_STDEXEC_OPERATIONS_DUALCAST_HPP

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

// Dualcast copies data from source to two destinations simultaneously.
// IMPORTANT: Both destination addresses must have the same bits 11:0
// (i.e., same offset within a 4KB page).

template <class DsaType, class ReceiverId>
class DualcastOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  DualcastOperation(DsaType &dsa, const void *src, void *dst1, void *dst2,
                    size_t size, Receiver r)
      : dsa_(dsa), src_(src), dst1_(dst1), dst2_(dst2), size_(size),
        r_(std::move(r)) {}

  DualcastOperation(DualcastOperation &&) = delete;

  void start() noexcept {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();

    // Validate dualcast alignment requirement: bits 11:0 must match
    auto d1 = reinterpret_cast<uintptr_t>(dst1_);
    auto d2 = reinterpret_cast<uintptr_t>(dst2_);
    if ((d1 & 0xFFF) != (d2 & 0xFFF)) {
      auto err = DsaError("dualcast: destination addresses must have same bits 11:0");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(std::move(err)));
      return;
    }

    memset(desc, 0, sizeof(*desc));

    desc->opcode = DSA_OPCODE_DUALCAST;
    desc->flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    desc->xfer_size = static_cast<uint32_t>(size_);
    desc->src_addr = reinterpret_cast<uint64_t>(src_);
    desc->dst_addr = reinterpret_cast<uint64_t>(dst1_);
    desc->dest2 = reinterpret_cast<uint64_t>(dst2_);
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
    DualcastOperation *op;
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
      desc->src_addr += comp->bytes_completed;
      desc->dst_addr += comp->bytes_completed;
      desc->dest2 += comp->bytes_completed;
      desc->xfer_size -= comp->bytes_completed;
      memset(comp, 0, sizeof(*comp));
      try {
        dsa_.submit(this, desc);
      } catch (...) {
        stdexec::set_error(std::move(r_), std::current_exception());
      }
    } else {
      auto err = DsaError(status, *comp, desc->opcode, "dualcast");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(std::move(err)));
    }
  }

private:
  DsaType &dsa_;
  const void *src_;
  void *dst1_;
  void *dst2_;
  size_t size_;
  Receiver r_;
};

template <class DsaType>
class DualcastSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  DualcastSender(DsaType &dsa, const void *src, void *dst1, void *dst2, size_t size)
      : dsa_(dsa), src_(src), dst1_(dst1), dst2_(dst2), size_(size) {}

  auto connect(stdexec::receiver auto &&r) {
    return DualcastOperation<DsaType, stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, src_, dst1_, dst2_, size_, std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  const void *src_;
  void *dst1_;
  void *dst2_;
  size_t size_;
};

template <class DsaType>
inline DualcastSender<DsaType> dsa_dualcast(DsaType &dsa, const void *src,
                                             void *dst1, void *dst2, size_t size) {
  return DualcastSender<DsaType>(dsa, src, dst1, dst2, size);
}

} // namespace dsa_stdexec

#endif
