#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_CRC_GEN_HPP
#define DSA_STDEXEC_OPERATIONS_CRC_GEN_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct CrcGenOperation : DsaOperationMixin {
  using result_type = uint32_t;
  static constexpr std::string_view op_name = "crc_gen";

  CrcGenOperation(DsaType &dsa, const void *src, size_t size, uint32_t seed,
                  auto &&r)
      : dsa_(dsa), src_(src), size_(size), seed_(seed), r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) {
    dsa::fill_crc_gen(d, src_, size_, seed_);
  }

  uint32_t extract_result(const dsa_completion_record &comp) {
    return static_cast<uint32_t>(comp.crc_val);
  }

  DsaType &dsa_;
  const void *src_;
  size_t size_;
  uint32_t seed_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using CrcGenSender = DsaOpSender<CrcGenOperation, DsaType, uint32_t,
                                 const void *, size_t, uint32_t>;

template <class DsaType>
inline auto dsa_crc_gen(DsaType &dsa, const void *src, size_t size,
                        uint32_t seed = 0) {
  return CrcGenSender<DsaType>(dsa, src, size, seed);
}

} // namespace dsa_stdexec

#endif
