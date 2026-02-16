#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_MEM_FILL_HPP
#define DSA_STDEXEC_OPERATIONS_MEM_FILL_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct MemFillOperation : DsaOperationMixin {
  using result_type = void;
  static constexpr std::string_view op_name = "mem_fill";

  MemFillOperation(DsaType &dsa, void *dst, size_t size, uint64_t pattern,
                   auto &&r)
      : dsa_(dsa), dst_(dst), size_(size), pattern_(pattern),
        r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) {
    dsa::fill_mem_fill(d, dst_, size_, pattern_);
  }

  DsaType &dsa_;
  void *dst_;
  size_t size_;
  uint64_t pattern_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using MemFillSender =
    DsaOpSender<MemFillOperation, DsaType, void, void *, size_t, uint64_t>;

template <class DsaType>
inline auto dsa_mem_fill(DsaType &dsa, void *dst, size_t size,
                         uint64_t pattern) {
  return MemFillSender<DsaType>(dsa, dst, size, pattern);
}

} // namespace dsa_stdexec

#endif
