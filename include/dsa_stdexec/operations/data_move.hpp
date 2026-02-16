#pragma once
#ifndef DSA_STDEXEC_DATA_MOVE_HPP
#define DSA_STDEXEC_DATA_MOVE_HPP

#include <dsa_stdexec/operations/operation_base_mixin.hpp>

namespace dsa_stdexec {

template <class DsaType, class ReceiverId>
struct DataMoveOperation : DsaOperationMixin {
  using result_type = void;
  static constexpr std::string_view op_name = "data_move";

  DataMoveOperation(DsaType &dsa, void *src, void *dst, size_t size, auto &&r)
      : dsa_(dsa), src_(src), dst_(dst), size_(size), r_(std::move(r)) {}

  void fill_descriptor(dsa_hw_desc &d) {
    dsa::fill_data_move(d, src_, dst_, size_);
  }

  DsaType &dsa_;
  void *src_;
  void *dst_;
  size_t size_;
  stdexec::__t<ReceiverId> r_;
};

template <class DsaType>
using DataMoveSender =
    DsaOpSender<DataMoveOperation, DsaType, void, void *, void *, size_t>;

template <class DsaType>
inline auto dsa_data_move(DsaType &dsa, void *src, void *dst, size_t size) {
  return DataMoveSender<DsaType>(dsa, src, dst, size);
}

} // namespace dsa_stdexec

#endif
