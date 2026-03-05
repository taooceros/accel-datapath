#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_COMPARE_VALUE_HPP
#define DSA_STDEXEC_OPERATIONS_COMPARE_VALUE_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct CompareValueOperation : DsaOperationMixin {
  using result_type = bool;
  static constexpr std::string_view op_name = "compare_value";

  CompareValueOperation(DsaType &dsa, const void *src, size_t size,
                        uint64_t pattern, auto &&r)
      : dsa_(dsa), src_(src), size_(size), pattern_(pattern),
        r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) {
    dsa::fill_compare_value(d, src_, size_, pattern_);
  }

  bool extract_result(const dsa_completion_record &comp) {
    return comp.result == 0;
  }

  DsaType &dsa_;
  const void *src_;
  size_t size_;
  uint64_t pattern_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using CompareValueSender = DsaOpSender<CompareValueOperation, DsaType, bool,
                                       const void *, size_t, uint64_t>;

template <class DsaType>
inline auto dsa_compare_value(DsaType &dsa, const void *src, size_t size,
                              uint64_t pattern) {
  return CompareValueSender<DsaType>(dsa, src, size, pattern);
}

} // namespace dsa_stdexec

#endif
