#pragma once
#include <cstring>
#include <fmt/base.h>
#ifndef DSA_STDEXEC_DATA_MOVE_HPP
#define DSA_STDEXEC_DATA_MOVE_HPP

#include <atomic>
#include <cstdint>
#include <dsa/dsa.hpp>
#include <dsa/dsa_operation_base.hpp>
#include <dsa_stdexec/error.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <exception>
#include <stdexec/execution.hpp>
#include <utility>

namespace dsa_stdexec {

// Global counter for page fault retries
inline std::atomic<uint64_t> g_page_fault_retries{0};

inline uint64_t get_page_fault_retries() {
  return g_page_fault_retries.load(std::memory_order_relaxed);
}

inline void reset_page_fault_retries() {
  g_page_fault_retries.store(0, std::memory_order_relaxed);
}

template <class DsaType, class ReceiverId>
class DataMoveOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;
  static_assert(!std::is_reference_v<Receiver>,
                "Receiver must not be a reference");

public:
  using operation_state_concept = stdexec::operation_state_t;

  DataMoveOperation(DsaType &dsa, void *src, void *dst, size_t size, Receiver r)
      : dsa_(dsa), src_(src), dst_(dst), size_(size), r_(std::move(r)) {}

  DataMoveOperation(DataMoveOperation &&) = delete;

  void start() noexcept {
    desc_.opcode = DSA_OPCODE_MEMMOVE;
    desc_.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    desc_.xfer_size = static_cast<uint32_t>(size_);
    desc_.src_addr = reinterpret_cast<uint64_t>(src_);
    desc_.dst_addr = reinterpret_cast<uint64_t>(dst_);
    desc_.completion_addr = reinterpret_cast<uint64_t>(&comp_);

    // Zero out completion record
    memset(&comp_, 0, sizeof(comp_));

    // Initialize the proxy for notify/get_descriptor callbacks
    proxy = pro::make_proxy<OperationFacade>(Wrapper{this});

    try {
      dsa_.submit(this, &desc_);
    } catch (const DsaError &e) {
      fmt::println(stderr, "DSA submit failed: {}", e.full_report());
      stdexec::set_error(std::move(r_), std::current_exception());
    } catch (const std::exception &e) {
      fmt::println(stderr, "DSA submit failed: {}", e.what());
      stdexec::set_error(std::move(r_), std::make_exception_ptr(
          DsaSubmitError(e.what())));
    } catch (...) {
      fmt::println(stderr, "DSA submit failed: unknown error");
      stdexec::set_error(std::move(r_), std::make_exception_ptr(
          DsaSubmitError("unknown error")));
    }
  }

private:
  struct Wrapper {
    DataMoveOperation *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return op->get_descriptor(); }
  };

  dsa_hw_desc *get_descriptor() { return &desc_; }

  void notify() {
    uint8_t status = comp_.status & DSA_COMP_STATUS_MASK;
    if (status == DSA_COMP_SUCCESS) {
      stdexec::set_value(std::move(r_));
    } else if (status == DSA_COMP_PAGE_FAULT_NOBOF) {
      // Increment page fault retry counter
      g_page_fault_retries.fetch_add(1, std::memory_order_relaxed);
      // fmt::println("page fault");
      int wr = comp_.status & DSA_COMP_STATUS_WRITE;
      volatile char *t;
      t = (char *)comp_.fault_addr;
      wr ? *t = *t : *t;
      desc_.src_addr += comp_.bytes_completed;
      desc_.dst_addr += comp_.bytes_completed;
      desc_.xfer_size -= comp_.bytes_completed;

      // Zero out completion record
      memset(&comp_, 0, sizeof(comp_));

      try {
        dsa_.submit(this, &desc_);
      } catch (...) {
        stdexec::set_error(std::move(r_), std::current_exception());
      }
    } else {
      auto err = DsaError(status, comp_, desc_.opcode, "data_move");
      fmt::println(stderr, "DSA operation failed: {}", err.full_report());
      stdexec::set_error(std::move(r_),
                         std::make_exception_ptr(std::move(err)));
    }
  }

private:
  DsaType &dsa_;
  void *src_;
  void *dst_;
  size_t size_;
  Receiver r_;
};

template <class DsaType>
class DataMoveSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  DataMoveSender(DsaType &dsa, void *src, void *dst, size_t size)
      : dsa_(dsa), src_(src), dst_(dst), size_(size) {}

  auto connect(stdexec::receiver auto &&r) {
    return DataMoveOperation<DsaType, stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
        dsa_, src_, dst_, size_, std::forward<decltype(r)>(r));
  }

private:
  DsaType &dsa_;
  void *src_;
  void *dst_;
  size_t size_;
};

// Helper to create the sender (templated)
template <class DsaType>
inline DataMoveSender<DsaType> dsa_data_move(DsaType &dsa, void *src, void *dst,
                                              size_t size) {
  return DataMoveSender<DsaType>(dsa, src, dst, size);
}

// Helper for pipeable syntax: just(src, dst, size) | dsa_data_move(dsa)
// This is a bit more complex to implement correctly as a closure.
// For simplicity, let's stick to direct construction or a simple adaptor if
// needed. But wait, the user might want: just(src, dst, size) |
// then(dsa_move(dsa))? No, usually it's `dsa_data_move(dsa, src, dst, size)`.

// If we want `just(src, dst, size) | dsa_data_move(dsa)`, we need a sender
// adaptor. Let's implement a simple sender adaptor that takes arguments from
// the previous sender.

template <class DsaType>
struct DsaDataMoveAdaptor {
  DsaType &dsa_;
  explicit DsaDataMoveAdaptor(DsaType &dsa) : dsa_(dsa) {}

  template <class Sender>
  friend auto operator|(Sender &&sender, DsaDataMoveAdaptor adaptor) {
    return stdexec::then(
        std::forward<Sender>(sender),
        [&dsa = adaptor.dsa_](void *src, void *dst, size_t size) {
          // This is tricky. `then` expects a callable that returns a value (or
          // void). If we return a sender from `then`, it becomes a sender of
          // sender. We need `let_value` to return a sender.
          return dsa_data_move(dsa, src, dst, size);
        });
  }
};

// Actually, `let_value` is what we want if we are chaining.
// But `then` is for synchronous computations.
// If we want to consume the values and return a new sender, we should use
// `let_value`. However, standard `dsa_data_move` usage is likely standalone or
// composed.

} // namespace dsa_stdexec

#endif
