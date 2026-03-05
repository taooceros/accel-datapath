#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_CACHE_FLUSH_HPP
#define DSA_STDEXEC_OPERATIONS_CACHE_FLUSH_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct CacheFlushOperation : DsaOperationMixin {
  using result_type = void;
  static constexpr std::string_view op_name = "cache_flush";

  CacheFlushOperation(DsaType &dsa, void *dst, size_t size, auto &&r)
      : dsa_(dsa), dst_(dst), size_(size), r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) { dsa::fill_cache_flush(d, dst_, size_); }

  DsaType &dsa_;
  void *dst_;
  size_t size_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using CacheFlushSender =
    DsaOpSender<CacheFlushOperation, DsaType, void, void *, size_t>;

template <class DsaType>
inline auto dsa_cache_flush(DsaType &dsa, void *dst, size_t size) {
  return CacheFlushSender<DsaType>(dsa, dst, size);
}

} // namespace dsa_stdexec

#endif
