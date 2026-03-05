#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_COMPARE_HPP
#define DSA_STDEXEC_OPERATIONS_COMPARE_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct CompareOperation : DsaOperationMixin {
  using result_type = bool;
  static constexpr std::string_view op_name = "compare";

  CompareOperation(DsaType &dsa, const void *src1, const void *src2,
                   size_t size, auto &&r)
      : dsa_(dsa), src1_(src1), src2_(src2), size_(size), r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) {
    dsa::fill_compare(d, src1_, src2_, size_);
  }

  bool extract_result(const dsa_completion_record &comp) {
    return comp.result == 0;
  }

  DsaType &dsa_;
  const void *src1_;
  const void *src2_;
  size_t size_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using CompareSender = DsaOpSender<CompareOperation, DsaType, bool,
                                  const void *, const void *, size_t>;

template <class DsaType>
inline auto dsa_compare(DsaType &dsa, const void *src1, const void *src2,
                        size_t size) {
  return CompareSender<DsaType>(dsa, src1, src2, size);
}

} // namespace dsa_stdexec

#endif
