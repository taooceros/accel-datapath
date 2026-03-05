#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_DUALCAST_HPP
#define DSA_STDEXEC_OPERATIONS_DUALCAST_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct DualcastOperation : DsaOperationMixin {
  using result_type = void;
  static constexpr std::string_view op_name = "dualcast";

  DualcastOperation(DsaType &dsa, const void *src, void *dst1, void *dst2,
                    size_t size, auto &&r)
      : dsa_(dsa), src_(src), dst1_(dst1), dst2_(dst2), size_(size),
        r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) {
    dsa::fill_dualcast(d, src_, dst1_, dst2_, size_);
  }

  bool pre_start_validate() {
    auto d1 = reinterpret_cast<uintptr_t>(dst1_);
    auto d2 = reinterpret_cast<uintptr_t>(dst2_);
    if ((d1 & 0xFFF) != (d2 & 0xFFF)) {
      auto err =
          DsaError("dualcast: destination addresses must have same bits 11:0");
      stdexec::set_error(std::move(r_),
                         std::make_exception_ptr(std::move(err)));
      return false;
    }
    return true;
  }

  DsaType &dsa_;
  const void *src_;
  void *dst1_;
  void *dst2_;
  size_t size_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using DualcastSender = DsaOpSender<DualcastOperation, DsaType, void,
                                   const void *, void *, void *, size_t>;

template <class DsaType>
inline auto dsa_dualcast(DsaType &dsa, const void *src, void *dst1, void *dst2,
                         size_t size) {
  return DualcastSender<DsaType>(dsa, src, dst1, dst2, size);
}

} // namespace dsa_stdexec

#endif
