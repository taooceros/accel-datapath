#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_OPERATION_BASE_MIXIN_HPP
#define DSA_STDEXEC_OPERATIONS_OPERATION_BASE_MIXIN_HPP

#include <atomic>
#include <concepts>
#include <cstdint>
#include <cstring>
#include <tuple>
#include <type_traits>
#include <utility>

#include <dsa/dsa.hpp>
#include <dsa/dsa_operation_base.hpp>
#include <dsa_stdexec/error.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <dsa_stdexec/batch.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>

namespace dsa_stdexec {

// ============================================================================
// Page fault retry counter (shared across all operations)
// ============================================================================

inline std::atomic<uint64_t> g_page_fault_retries{0};

inline uint64_t get_page_fault_retries() {
  return g_page_fault_retries.load(std::memory_order_relaxed);
}

inline void reset_page_fault_retries() {
  g_page_fault_retries.store(0, std::memory_order_relaxed);
}

// ============================================================================
// adjust_for_page_fault — opcode-dispatched page fault handler
// ============================================================================

inline void adjust_for_page_fault(dsa_hw_desc &desc,
                                  const dsa_completion_record &comp) {
  // Touch the faulting page
  int wr = comp.status & DSA_COMP_STATUS_WRITE;
  volatile char *t = (char *)comp.fault_addr;
  if (wr) { *t = *t; } else { (void)*t; }

  // Adjust descriptor fields based on opcode
  switch (desc.opcode) {
  case DSA_OPCODE_MEMMOVE:
    desc.src_addr += comp.bytes_completed;
    desc.dst_addr += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;
    break;
  case DSA_OPCODE_COPY_CRC:
    desc.crc_seed = static_cast<uint32_t>(comp.crc_val);
    desc.src_addr += comp.bytes_completed;
    desc.dst_addr += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;
    break;
  case DSA_OPCODE_MEMFILL:
  case DSA_OPCODE_CFLUSH:
    desc.dst_addr += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;
    break;
  case DSA_OPCODE_COMPARE:
    desc.src_addr += comp.bytes_completed;
    desc.src2_addr += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;
    break;
  case DSA_OPCODE_COMPVAL:
    desc.src_addr += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;
    break;
  case DSA_OPCODE_DUALCAST:
    desc.src_addr += comp.bytes_completed;
    desc.dst_addr += comp.bytes_completed;
    desc.dest2 += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;
    break;
  case DSA_OPCODE_CRCGEN:
    desc.crc_seed = static_cast<uint32_t>(comp.crc_val);
    desc.src_addr += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;
    break;
  default:
    break;
  }
}

// ============================================================================
// DsaOperation concept — constrains derived operation types
// ============================================================================

template <typename T>
concept DsaOperation =
    std::derived_from<T, dsa::DsaOperationBase> &&
    requires(T &op, dsa_hw_desc &desc) {
      typename T::result_type;
      { T::op_name } -> std::convertible_to<std::string_view>;
      { op.fill_descriptor(desc) } -> std::same_as<void>;
      op.dsa_;
      op.r_;
    };

template <typename T>
concept DsaOperationWithResult =
    DsaOperation<T> && !std::is_void_v<typename T::result_type> &&
    requires(T &op, const dsa_completion_record &comp) {
      { op.extract_result(comp) } -> std::same_as<typename T::result_type>;
    };

// ============================================================================
// DsaOperationMixin — non-templated base using C++23 deducing this
// ============================================================================

struct DsaOperationMixin : dsa::DsaOperationBase {
  using operation_state_concept = stdexec::operation_state_t;

  DsaOperationMixin() = default;
  DsaOperationMixin(DsaOperationMixin &&) = delete;

  void start(this DsaOperation auto &self) noexcept {
    auto *desc = self.desc_ptr();
    auto *comp = self.comp_ptr();
    memset(desc, 0, sizeof(*desc));
    memset(comp, 0, sizeof(*comp));

    // Optional hook: pre-start validation (e.g. dualcast alignment)
    if constexpr (requires { self.pre_start_validate(); }) {
      if (!self.pre_start_validate())
        return;
    }

    self.fill_descriptor(*desc);
    desc->completion_addr = reinterpret_cast<uint64_t>(comp);

    // Wrapper for type-erased proxy callbacks
    using Self = std::remove_reference_t<decltype(self)>;
    struct Wrapper {
      Self *op;
      void notify() { op->notify(); }
      dsa_hw_desc *get_descriptor() { return op->desc_ptr(); }
    };
    self.proxy = pro::make_proxy<OperationFacade>(Wrapper{&self});

    try {
      self.dsa_.submit(&self, desc);
    } catch (const DsaError &e) {
      fmt::println(stderr, "DSA submit failed: {}", e.full_report());
      stdexec::set_error(std::move(self.r_), std::current_exception());
    } catch (const std::exception &e) {
      fmt::println(stderr, "DSA submit failed: {}", e.what());
      stdexec::set_error(std::move(self.r_),
                         std::make_exception_ptr(DsaSubmitError(e.what())));
    } catch (...) {
      fmt::println(stderr, "DSA submit failed: unknown error");
      stdexec::set_error(
          std::move(self.r_),
          std::make_exception_ptr(DsaSubmitError("unknown error")));
    }
  }

  void notify(this DsaOperation auto &self) {
    using Self = std::remove_reference_t<decltype(self)>;
    if constexpr (!std::is_void_v<typename Self::result_type>) {
      static_assert(DsaOperationWithResult<Self>,
                    "Non-void operation must provide extract_result()");
    }

    auto *desc = self.desc_ptr();
    auto *comp = self.comp_ptr();
    uint8_t status = comp->status & DSA_COMP_STATUS_MASK;

    if constexpr (std::is_void_v<typename Self::result_type>) {
      if (status == DSA_COMP_SUCCESS) {
        stdexec::set_value(std::move(self.r_));
        return;
      }
    } else {
      if (status == DSA_COMP_SUCCESS || status == DSA_COMP_SUCCESS_PRED) {
        stdexec::set_value(std::move(self.r_), self.extract_result(*comp));
        return;
      }
    }

    if (status == DSA_COMP_PAGE_FAULT_NOBOF) {
      g_page_fault_retries.fetch_add(1, std::memory_order_relaxed);
      adjust_for_page_fault(*desc, *comp);
      memset(comp, 0, sizeof(*comp));
      try {
        self.dsa_.submit(&self, desc);
      } catch (...) {
        stdexec::set_error(std::move(self.r_), std::current_exception());
      }
      return;
    }

    auto err = DsaError(status, *comp, desc->opcode, Self::op_name);
    stdexec::set_error(std::move(self.r_),
                       std::make_exception_ptr(std::move(err)));
  }
};

// ============================================================================
// DsaOpSender — generic sender eliminating per-operation Sender classes
// ============================================================================

namespace detail {
template <typename ResultType>
struct completion_sigs_for {
  using type = stdexec::completion_signatures<stdexec::set_value_t(ResultType),
                                              stdexec::set_error_t(std::exception_ptr)>;
};
template <>
struct completion_sigs_for<void> {
  using type = stdexec::completion_signatures<stdexec::set_value_t(),
                                              stdexec::set_error_t(std::exception_ptr)>;
};
} // namespace detail

template <typename ResultType>
using dsa_completion_sigs = typename detail::completion_sigs_for<ResultType>::type;

template <template <class, class> class OpTmpl, class DsaType,
          typename ResultType, typename... Params>
class DsaOpSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures = dsa_completion_sigs<ResultType>;

  DsaOpSender(DsaType &dsa, Params... params)
      : dsa_(dsa), params_(params...) {}

  auto connect(stdexec::receiver auto &&r) {
    return std::apply(
        [&](auto... args) {
          return OpTmpl<DsaType,
                        stdexec::__id<std::remove_cvref_t<decltype(r)>>>(
              dsa_, args..., std::forward<decltype(r)>(r));
        },
        params_);
  }

private:
  DsaType &dsa_;
  std::tuple<Params...> params_;
};

} // namespace dsa_stdexec

#endif
