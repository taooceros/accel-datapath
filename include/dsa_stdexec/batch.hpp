#pragma once
#ifndef DSA_STDEXEC_BATCH_HPP
#define DSA_STDEXEC_BATCH_HPP

#include <cstdint>
#include <cstring>
#include <span>
#include <utility>

#include <dsa/dsa.hpp>
#include <dsa/dsa_operation_base.hpp>
#include <dsa_stdexec/descriptor_fill.hpp>
#include <dsa_stdexec/error.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>

namespace dsa_stdexec {

// ============================================================================
// BatchOperation — operation state for a hardware batch descriptor
// ============================================================================

static constexpr size_t kMaxBatch = 32;

template <class DsaType, class Factory, class ReceiverId>
class BatchOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  BatchOperation(DsaType &dsa, size_t count, Factory factory, Receiver r)
      : dsa_(dsa), count_(count), factory_(std::move(factory)),
        downstream_(std::move(r)) {}

  BatchOperation(BatchOperation &&) = delete;

  void start() noexcept {
    // Zero sub-descriptors and sub-completion records
    memset(sub_descs_, 0, count_ * sizeof(dsa_hw_desc));
    memset(sub_comps_, 0, count_ * sizeof(dsa_completion_record));

    // Set completion_addr for each sub-descriptor
    for (size_t i = 0; i < count_; ++i) {
      sub_descs_[i].completion_addr =
          reinterpret_cast<uint64_t>(&sub_comps_[i]);
    }

    // Let the factory fill opcode-specific fields
    factory_(std::span<dsa_hw_desc>{sub_descs_, count_});

    // Build the batch descriptor (in inherited DsaOperationBase storage)
    auto *desc = desc_ptr();
    auto *comp = comp_ptr();

    memset(desc, 0, sizeof(*desc));
    desc->opcode = DSA_OPCODE_BATCH;
    desc->flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    desc->desc_list_addr = reinterpret_cast<uint64_t>(&sub_descs_[0]);
    desc->desc_count = static_cast<uint16_t>(count_);
    desc->completion_addr = reinterpret_cast<uint64_t>(comp);

    memset(comp, 0, sizeof(*comp));

    // Initialize proxy for notify/get_descriptor callbacks
    proxy = pro::make_proxy<OperationFacade>(Wrapper{this});

    try {
      dsa_.submit(this, desc);
    } catch (const DsaError &e) {
      fmt::println(stderr, "DSA batch submit failed: {}", e.full_report());
      stdexec::set_error(std::move(downstream_), std::current_exception());
    } catch (const std::exception &e) {
      fmt::println(stderr, "DSA batch submit failed: {}", e.what());
      stdexec::set_error(std::move(downstream_),
                         std::make_exception_ptr(DsaSubmitError(e.what())));
    } catch (...) {
      fmt::println(stderr, "DSA batch submit failed: unknown error");
      stdexec::set_error(
          std::move(downstream_),
          std::make_exception_ptr(DsaSubmitError("unknown error")));
    }
  }

private:
  struct Wrapper {
    BatchOperation *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return op->desc_ptr(); }
  };

  void notify() {
    uint8_t status = comp_ptr()->status & DSA_COMP_STATUS_MASK;
    if (status == DSA_COMP_SUCCESS) {
      stdexec::set_value(std::move(downstream_));
    } else {
      auto err = DsaError(status, *comp_ptr(), DSA_OPCODE_BATCH, "batch");
      fmt::println(stderr, "DSA batch failed: {}", err.full_report());
      stdexec::set_error(std::move(downstream_),
                         std::make_exception_ptr(std::move(err)));
    }
  }

  DsaType &dsa_;
  Factory factory_;
  size_t count_;
  Receiver downstream_;

  alignas(64) dsa_hw_desc sub_descs_[kMaxBatch];
  alignas(32) dsa_completion_record sub_comps_[kMaxBatch];
};

// ============================================================================
// BatchSender
// ============================================================================

template <class DsaType, class Factory> class BatchSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  BatchSender(DsaType &dsa, size_t count, Factory factory)
      : dsa_(dsa), count_(count), factory_(std::move(factory)) {}

  auto connect(stdexec::receiver auto &&r) {
    return BatchOperation<DsaType, Factory,
                          stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, count_, std::move(factory_), std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  size_t count_;
  Factory factory_;
};

// ============================================================================
// dsa_batch free function
// ============================================================================

template <class DsaType, class Factory>
auto dsa_batch(DsaType &dsa, size_t count, Factory &&factory) {
  return BatchSender<DsaType, std::decay_t<Factory>>(
      dsa, count, std::forward<Factory>(factory));
}

} // namespace dsa_stdexec

#endif
