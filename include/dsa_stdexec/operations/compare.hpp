#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_COMPARE_HPP
#define DSA_STDEXEC_OPERATIONS_COMPARE_HPP

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
class CompareOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  CompareOperation(DsaType &dsa, const void *src1, const void *src2, size_t size, Receiver r)
      : dsa_(dsa), src1_(src1), src2_(src2), size_(size), r_(std::move(r)) {}

  CompareOperation(CompareOperation &&) = delete;

  void start() noexcept {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();

    memset(desc, 0, sizeof(*desc));

    desc->opcode = DSA_OPCODE_COMPARE;
    // Note: Do NOT use IDXD_OP_FLAG_CC for compare - it means "check against expected_res"
    desc->flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    desc->xfer_size = static_cast<uint32_t>(size_);
    desc->src_addr = reinterpret_cast<uint64_t>(src1_);
    desc->src2_addr = reinterpret_cast<uint64_t>(src2_);
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
    CompareOperation *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return op->get_descriptor(); }
  };

  dsa_hw_desc *get_descriptor() { return desc_ptr(); }

  void notify() {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();
    uint8_t status = comp->status & DSA_COMP_STATUS_MASK;
    if (status == DSA_COMP_SUCCESS || status == DSA_COMP_SUCCESS_PRED) {
      // result == 0 means regions are equal, result == 1 means mismatch
      bool equal = (comp->result == 0);
      stdexec::set_value(std::move(r_), equal);
    } else if (status == DSA_COMP_PAGE_FAULT_NOBOF) {
      int wr = comp->status & DSA_COMP_STATUS_WRITE;
      volatile char *t = (char *)comp->fault_addr;
      wr ? *t = *t : *t;
      desc->src_addr += comp->bytes_completed;
      desc->src2_addr += comp->bytes_completed;
      desc->xfer_size -= comp->bytes_completed;
      memset(comp, 0, sizeof(*comp));
      try {
        dsa_.submit(this, desc);
      } catch (...) {
        stdexec::set_error(std::move(r_), std::current_exception());
      }
    } else {
      auto err = DsaError(status, *comp, desc->opcode, "compare");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(std::move(err)));
    }
  }

private:
  DsaType &dsa_;
  const void *src1_;
  const void *src2_;
  size_t size_;
  Receiver r_;
};

template <class DsaType>
class CompareSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(bool),
                                     stdexec::set_error_t(std::exception_ptr)>;

  CompareSender(DsaType &dsa, const void *src1, const void *src2, size_t size)
      : dsa_(dsa), src1_(src1), src2_(src2), size_(size) {}

  auto connect(stdexec::receiver auto &&r) {
    return CompareOperation<DsaType, stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, src1_, src2_, size_, std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  const void *src1_;
  const void *src2_;
  size_t size_;
};

template <class DsaType>
inline CompareSender<DsaType> dsa_compare(DsaType &dsa, const void *src1,
                                           const void *src2, size_t size) {
  return CompareSender<DsaType>(dsa, src1, src2, size);
}

} // namespace dsa_stdexec

#endif
