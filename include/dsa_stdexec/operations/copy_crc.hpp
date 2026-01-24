#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_COPY_CRC_HPP
#define DSA_STDEXEC_OPERATIONS_COPY_CRC_HPP

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

// Copy with CRC generation using Intel DSA hardware.
// Copies data from source to destination while computing CRC-32C.

template <class DsaType, class ReceiverId>
class CopyCrcOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  CopyCrcOperation(DsaType &dsa, const void *src, void *dst, size_t size,
                   uint32_t seed, Receiver r)
      : dsa_(dsa), src_(src), dst_(dst), size_(size), seed_(seed),
        r_(std::move(r)) {}

  CopyCrcOperation(CopyCrcOperation &&) = delete;

  void start() noexcept {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();

    memset(desc, 0, sizeof(*desc));

    desc->opcode = DSA_OPCODE_COPY_CRC;
    desc->flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    desc->xfer_size = static_cast<uint32_t>(size_);
    desc->src_addr = reinterpret_cast<uint64_t>(src_);
    desc->dst_addr = reinterpret_cast<uint64_t>(dst_);
    desc->crc_seed = seed_;
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
    CopyCrcOperation *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return op->get_descriptor(); }
  };

  dsa_hw_desc *get_descriptor() { return desc_ptr(); }

  void notify() {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();
    uint8_t status = comp->status & DSA_COMP_STATUS_MASK;
    if (status == DSA_COMP_SUCCESS) {
      uint32_t crc = static_cast<uint32_t>(comp->crc_val);
      stdexec::set_value(std::move(r_), crc);
    } else if (status == DSA_COMP_PAGE_FAULT_NOBOF) {
      int wr = comp->status & DSA_COMP_STATUS_WRITE;
      volatile char *t = (char *)comp->fault_addr;
      wr ? *t = *t : *t;
      // Continue with partial CRC as new seed
      desc->crc_seed = static_cast<uint32_t>(comp->crc_val);
      desc->src_addr += comp->bytes_completed;
      desc->dst_addr += comp->bytes_completed;
      desc->xfer_size -= comp->bytes_completed;
      memset(comp, 0, sizeof(*comp));
      try {
        dsa_.submit(this, desc);
      } catch (...) {
        stdexec::set_error(std::move(r_), std::current_exception());
      }
    } else {
      auto err = DsaError(status, *comp, desc->opcode, "copy_crc");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(std::move(err)));
    }
  }

private:
  DsaType &dsa_;
  const void *src_;
  void *dst_;
  size_t size_;
  uint32_t seed_;
  Receiver r_;
};

template <class DsaType>
class CopyCrcSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(uint32_t),
                                     stdexec::set_error_t(std::exception_ptr)>;

  CopyCrcSender(DsaType &dsa, const void *src, void *dst, size_t size,
                uint32_t seed = 0)
      : dsa_(dsa), src_(src), dst_(dst), size_(size), seed_(seed) {}

  auto connect(stdexec::receiver auto &&r) {
    return CopyCrcOperation<DsaType, stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, src_, dst_, size_, seed_, std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  const void *src_;
  void *dst_;
  size_t size_;
  uint32_t seed_;
};

template <class DsaType>
inline CopyCrcSender<DsaType> dsa_copy_crc(DsaType &dsa, const void *src,
                                            void *dst, size_t size,
                                            uint32_t seed = 0) {
  return CopyCrcSender<DsaType>(dsa, src, dst, size, seed);
}

} // namespace dsa_stdexec

#endif
