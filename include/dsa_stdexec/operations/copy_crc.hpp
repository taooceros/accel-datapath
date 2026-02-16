#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_COPY_CRC_HPP
#define DSA_STDEXEC_OPERATIONS_COPY_CRC_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct CopyCrcOperation : DsaOperationMixin {
  using result_type = uint32_t;
  static constexpr std::string_view op_name = "copy_crc";

  CopyCrcOperation(DsaType &dsa, const void *src, void *dst, size_t size,
                   uint32_t seed, auto &&r)
      : dsa_(dsa), src_(src), dst_(dst), size_(size), seed_(seed),
        r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) {
    dsa::fill_copy_crc(d, src_, dst_, size_, seed_);
  }

  uint32_t extract_result(const dsa_completion_record &comp) {
    return static_cast<uint32_t>(comp.crc_val);
  }

  DsaType &dsa_;
  const void *src_;
  void *dst_;
  size_t size_;
  uint32_t seed_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using CopyCrcSender = DsaOpSender<CopyCrcOperation, DsaType, uint32_t,
                                  const void *, void *, size_t, uint32_t>;

template <class DsaType>
inline auto dsa_copy_crc(DsaType &dsa, const void *src, void *dst, size_t size,
                         uint32_t seed = 0) {
  return CopyCrcSender<DsaType>(dsa, src, dst, size, seed);
}

} // namespace dsa_stdexec

#endif
