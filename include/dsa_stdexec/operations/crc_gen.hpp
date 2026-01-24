#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_CRC_GEN_HPP
#define DSA_STDEXEC_OPERATIONS_CRC_GEN_HPP

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

// CRC generation using Intel DSA hardware.
// Generates CRC-32C (Castagnoli polynomial) over the source buffer.

template <class DsaType, class ReceiverId>
class CrcGenOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  CrcGenOperation(DsaType &dsa, const void *src, size_t size, uint32_t seed, Receiver r)
      : dsa_(dsa), src_(src), size_(size), seed_(seed), r_(std::move(r)) {}

  CrcGenOperation(CrcGenOperation &&) = delete;

  void start() noexcept {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();

    memset(desc, 0, sizeof(*desc));

    desc->opcode = DSA_OPCODE_CRCGEN;
    desc->flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    desc->xfer_size = static_cast<uint32_t>(size_);
    desc->src_addr = reinterpret_cast<uint64_t>(src_);
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
    CrcGenOperation *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return op->get_descriptor(); }
  };

  dsa_hw_desc *get_descriptor() { return desc_ptr(); }

  void notify() {
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();
    uint8_t status = comp->status & DSA_COMP_STATUS_MASK;
    if (status == DSA_COMP_SUCCESS) {
      // CRC result is in the completion record (lower 32 bits)
      uint32_t crc = static_cast<uint32_t>(comp->crc_val);
      stdexec::set_value(std::move(r_), crc);
    } else if (status == DSA_COMP_PAGE_FAULT_NOBOF) {
      int wr = comp->status & DSA_COMP_STATUS_WRITE;
      volatile char *t = (char *)comp->fault_addr;
      wr ? *t = *t : *t;
      // For CRC, we need to continue with partial CRC as new seed
      desc->crc_seed = static_cast<uint32_t>(comp->crc_val);
      desc->src_addr += comp->bytes_completed;
      desc->xfer_size -= comp->bytes_completed;
      memset(comp, 0, sizeof(*comp));
      try {
        dsa_.submit(this, desc);
      } catch (...) {
        stdexec::set_error(std::move(r_), std::current_exception());
      }
    } else {
      auto err = DsaError(status, *comp, desc->opcode, "crc_gen");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(std::move(err)));
    }
  }

private:
  DsaType &dsa_;
  const void *src_;
  size_t size_;
  uint32_t seed_;
  Receiver r_;
};

template <class DsaType>
class CrcGenSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(uint32_t),
                                     stdexec::set_error_t(std::exception_ptr)>;

  CrcGenSender(DsaType &dsa, const void *src, size_t size, uint32_t seed = 0)
      : dsa_(dsa), src_(src), size_(size), seed_(seed) {}

  auto connect(stdexec::receiver auto &&r) {
    return CrcGenOperation<DsaType, stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, src_, size_, seed_, std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  const void *src_;
  size_t size_;
  uint32_t seed_;
};

template <class DsaType>
inline CrcGenSender<DsaType> dsa_crc_gen(DsaType &dsa, const void *src,
                                          size_t size, uint32_t seed = 0) {
  return CrcGenSender<DsaType>(dsa, src, size, seed);
}

} // namespace dsa_stdexec

#endif
