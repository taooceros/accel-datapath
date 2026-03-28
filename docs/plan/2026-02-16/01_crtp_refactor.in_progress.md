# Refactor: Deduplicate Operation Senders via Deducing `this` (C++23)

## Context

The 8 DSA operation sender files (`data_move.hpp`, `mem_fill.hpp`, `compare.hpp`,
`compare_value.hpp`, `dualcast.hpp`, `crc_gen.hpp`, `copy_crc.hpp`, `cache_flush.hpp`)
each contain ~100 lines following an identical pattern. Only ~15-25 lines per file are
truly operation-specific. The remaining ~85% is duplicated boilerplate: `Wrapper` struct,
`get_descriptor()`, `start()` scaffolding (memset + proxy + submit + catch),
`notify()` scaffolding (status check + page fault touch + resubmit + error), `Sender`
class, and free function factory.

This refactoring extracts the boilerplate into a **non-templated** mixin base using
C++23 deducing `this` (P0847), plus a generic sender template. Each operation file
reduces to ~20-30 lines of unique code. We also introduce a `DsaSink` concept and a
shared `adjust_for_page_fault()` function.

### Why deducing `this` over traditional CRTP

| Aspect | Traditional CRTP | Deducing `this` (C++23) |
|--------|-----------------|------------------------|
| Base class | `template <Derived, DsaType, ReceiverId>` | **Non-templated** |
| Derived declaration | `class Foo : public Base<Foo, D, R>` | `class Foo : public DsaOperationBase` |
| Access to derived | `static_cast<Derived&>(*this)` | `self` parameter — already the right type |
| `friend` needed | Yes | **No** |
| Readability | Verbose | Clean, looks like regular methods |

GCC 15 (our compiler) fully supports deducing `this`.

## Files Overview

| File | Action | Description |
|------|--------|-------------|
| `include/dsa_stdexec/operations/operation_base_mixin.hpp` | **CREATE** | Mixin with deducing `this`, generic sender, page fault helper, counter |
| `include/dsa_stdexec/dsa_sink.hpp` | **CREATE** | `DsaSink` concept |
| `include/dsa_stdexec/operations/data_move.hpp` | MODIFY | ~130 lines → ~30 lines |
| `include/dsa_stdexec/operations/mem_fill.hpp` | MODIFY | ~100 lines → ~25 lines |
| `include/dsa_stdexec/operations/cache_flush.hpp` | MODIFY | ~95 lines → ~25 lines |
| `include/dsa_stdexec/operations/compare.hpp` | MODIFY | ~100 lines → ~30 lines |
| `include/dsa_stdexec/operations/compare_value.hpp` | MODIFY | ~100 lines → ~30 lines |
| `include/dsa_stdexec/operations/crc_gen.hpp` | MODIFY | ~100 lines → ~30 lines |
| `include/dsa_stdexec/operations/copy_crc.hpp` | MODIFY | ~110 lines → ~30 lines |
| `include/dsa_stdexec/operations/dualcast.hpp` | MODIFY | ~115 lines → ~35 lines |
| `include/dsa_stdexec/operations/all.hpp` | UNCHANGED | |

## Design

### 1. `DsaSink` concept (`include/dsa_stdexec/dsa_sink.hpp`)

Formalizes the interface that `DsaBase`, `DsaBatchBase`, `DsaRingBatchBase`, and
`DsaFixedRingBatchBase` all satisfy via duck-typing today:

```cpp
template <typename T>
concept DsaSink = requires(T &dsa, OperationBase *op, dsa_hw_desc *desc) {
    { dsa.submit(op, desc) } -> std::same_as<void>;
    { dsa.submit(op) } -> std::same_as<void>;
    { dsa.poll() };
};
```

Lightweight constraint — `flush()` omitted since not all call sites use it.

### 2. `adjust_for_page_fault()` free function

Opcode-dispatched page fault handler replacing 8 inline implementations. Located in
`operation_base_mixin.hpp`:

```cpp
inline void adjust_for_page_fault(dsa_hw_desc &desc,
                                  const dsa_completion_record &comp) {
    // Touch the faulting page
    int wr = comp.status & DSA_COMP_STATUS_WRITE;
    volatile char *t = reinterpret_cast<volatile char *>(comp.fault_addr);
    wr ? *t = *t : (void)*t;

    // Adjust fields based on opcode
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
    }
}
```

### 3. Page fault counter (relocated)

`g_page_fault_retries`, `get_page_fault_retries()`, and `reset_page_fault_retries()`
move from `data_move.hpp` to `operation_base_mixin.hpp` so all operations share one
counter.

### 4. `DsaOperation` concept — constraining derived types

Formalizes the contract that every derived operation must satisfy. Checked at the
deducing `this` call site, giving clear errors like "DataMoveOperation does not
satisfy DsaOperation" instead of deep template instantiation failures:

```cpp
// Core concept: what every DSA operation must provide
template <typename T>
concept DsaOperation = std::derived_from<T, dsa::DsaOperationBase>
    && requires(T &op, dsa_hw_desc &desc) {
        // Required type alias
        typename T::result_type;

        // Required static member
        { T::op_name } -> std::convertible_to<std::string_view>;

        // Required methods
        { op.fill_descriptor(desc) } -> std::same_as<void>;

        // Required data members
        op.dsa_;
        op.r_;
    };

// Refined concept for operations that produce a result value
template <typename T>
concept DsaOperationWithResult = DsaOperation<T>
    && !std::is_void_v<typename T::result_type>
    && requires(T &op, const dsa_completion_record &comp) {
        { op.extract_result(comp) } -> std::same_as<typename T::result_type>;
    };
```

Used in the mixin via constrained deducing `this`:
- `start(this DsaOperation auto &self)` — all operations
- `notify(this DsaOperation auto &self)` — with internal `static_assert` for
  `DsaOperationWithResult` when `result_type` is non-void

### 5. `DsaOperationMixin` — non-templated base with deducing `this`

The core of the refactoring. **No template parameters on the base class.** The
`this DsaOperation auto &self` parameter deduces the derived type at each call
site while enforcing the concept constraint:

```cpp
struct DsaOperationMixin : dsa::DsaOperationBase {
    using operation_state_concept = stdexec::operation_state_t;

    DsaOperationMixin(DsaOperationMixin &&) = delete;
    DsaOperationMixin() = default;

    // ── start() ─────────────────────────────────────────────────
    void start(this DsaOperation auto &self) noexcept {
        auto *desc = self.desc_ptr();
        auto *comp = self.comp_ptr();
        memset(desc, 0, sizeof(*desc));
        memset(comp, 0, sizeof(*comp));

        // Optional hook: pre-start validation (dualcast alignment)
        if constexpr (requires { self.pre_start_validate(); }) {
            if (!self.pre_start_validate()) return;
        }

        // Derived fills opcode-specific fields
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
            stdexec::set_error(std::move(self.r_),
                std::make_exception_ptr(DsaSubmitError("unknown error")));
        }
    }

    // ── notify() ────────────────────────────────────────────────
    void notify(this DsaOperation auto &self) {
        using Self = std::remove_reference_t<decltype(self)>;
        // Compile-time check: non-void ops must provide extract_result
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
                stdexec::set_value(std::move(self.r_),
                                   self.extract_result(*comp));
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
                stdexec::set_error(std::move(self.r_),
                                   std::current_exception());
            }
            return;
        }

        auto err = DsaError(status, *comp, desc->opcode, Self::op_name);
        stdexec::set_error(std::move(self.r_),
                           std::make_exception_ptr(std::move(err)));
    }
};
```

**Derived class contract** (what each operation must provide):

| Member | Type | Required | Purpose |
|--------|------|----------|---------|
| `result_type` | type alias | Yes | `void`, `bool`, or `uint32_t` |
| `op_name` | `static constexpr string_view` | Yes | Error messages |
| `dsa_` | `DsaType &` | Yes | DSA device reference |
| `r_` | `Receiver` | Yes | stdexec receiver |
| `fill_descriptor(dsa_hw_desc &)` | method | Yes | Fill opcode-specific fields |
| `extract_result(const dsa_completion_record &)` | method | Non-void only | Extract result value |
| `pre_start_validate()` | method | No (optional) | Pre-submit validation |

### 6. `DsaOpSender<OpTmpl, DsaType, ResultType, Params...>` (generic sender)

Eliminates the 8 near-identical Sender classes:

```cpp
template <typename ResultType>
using dsa_completion_sigs = stdexec::completion_signatures<
    std::conditional_t<std::is_void_v<ResultType>,
                       stdexec::set_value_t(),
                       stdexec::set_value_t(ResultType)>,
    stdexec::set_error_t(std::exception_ptr)>;

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
```

### 7. What each operation file looks like after refactoring

**Example: `data_move.hpp` (~25 lines, void result)**

```cpp
#pragma once
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
```

**Example: `compare.hpp` (~30 lines, bool result)**

```cpp
template <class DsaType, class ReceiverId>
struct CompareOperation : DsaOperationMixin {
    using result_type = bool;
    static constexpr std::string_view op_name = "compare";

    CompareOperation(DsaType &dsa, const void *src1, const void *src2,
                     size_t size, auto &&r)
        : dsa_(dsa), src1_(src1), src2_(src2), size_(size), r_(std::move(r)) {}

    void fill_descriptor(dsa_hw_desc &d) {
        dsa::fill_compare(d, src1_, src2_, size_);
    }

    bool extract_result(const dsa_completion_record &comp) {
        return comp.result == 0;
    }

    DsaType &dsa_;
    const void *src1_;
    const void *src2_;
    size_t size_;
    stdexec::__t<ReceiverId> r_;
};
```

**Example: `dualcast.hpp` (~35 lines, with pre_start_validate)**

```cpp
template <class DsaType, class ReceiverId>
struct DualcastOperation : DsaOperationMixin {
    using result_type = void;
    static constexpr std::string_view op_name = "dualcast";

    DualcastOperation(DsaType &dsa, const void *src, void *dst1, void *dst2,
                      size_t size, auto &&r)
        : dsa_(dsa), src_(src), dst1_(dst1), dst2_(dst2), size_(size),
          r_(std::move(r)) {}

    void fill_descriptor(dsa_hw_desc &d) {
        dsa::fill_dualcast(d, src_, dst1_, dst2_, size_);
    }

    bool pre_start_validate() {
        auto d1 = reinterpret_cast<uintptr_t>(dst1_);
        auto d2 = reinterpret_cast<uintptr_t>(dst2_);
        if ((d1 & 0xFFF) != (d2 & 0xFFF)) {
            auto err = DsaError(
                "dualcast: destination addresses must have same bits 11:0");
            stdexec::set_error(std::move(r_),
                               std::make_exception_ptr(std::move(err)));
            return false;
        }
        return true;
    }

    DsaType &dsa_;
    const void *src_;
    void *dst1_;
    void *dst2_;
    size_t size_;
    stdexec::__t<ReceiverId> r_;
};
```

Note: derived classes use `struct` (public by default) — no `friend` needed since
deducing `this` methods in the base access members directly through the `self`
parameter which is already the derived type.

### 8. Dead code cleanup

Remove `DsaDataMoveAdaptor` from `data_move.hpp` — experimental adaptor code with
TODO comments, never used.

## Implementation Order

1. **Create `dsa_sink.hpp`** — standalone, no dependencies
2. **Create `operation_base_mixin.hpp`** — mixin, generic sender, `adjust_for_page_fault`, counter
3. **Refactor `data_move.hpp`** — simplest void-result; build + run `example_data_move`
4. **Refactor `mem_fill.hpp` + `cache_flush.hpp`** — void-result, simple params
5. **Refactor `compare.hpp` + `compare_value.hpp`** — bool-result, adds `extract_result`
6. **Refactor `crc_gen.hpp` + `copy_crc.hpp`** — uint32_t-result
7. **Refactor `dualcast.hpp`** — void-result + `pre_start_validate` hook
8. **Full build + test** — all targets, all examples, benchmark

## Verification

```bash
# Build all targets
xmake f -m release && xmake

# Run each example via dsa_launcher
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_data_move
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_mem_fill
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_compare
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_compare_value
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_dualcast
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_crc_gen
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_copy_crc
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_cache_flush

# Benchmark
run

# AddressSanitizer
xmake f -m debug --policies=build.sanitizer.address && xmake
./build/linux/x86_64/debug/dsa_launcher ./build/linux/x86_64/debug/example_data_move
```

## Risk Notes

- **Deducing `this` + proxy**: The `Wrapper` struct is now defined locally inside
  `start()` since it needs the concrete `Self` type. This is fine — the proxy stores
  the wrapper by value, keeping it alive for the operation's lifetime.
- **`if constexpr` on `Self::result_type`**: Works because `Self` is the fully
  deduced derived type, so `Self::result_type` is always available.
- **`SUCCESS_PRED` handling**: Only compare/compare_value produce `SUCCESS_PRED`.
  Non-void ops check `SUCCESS || SUCCESS_PRED`. CRC ops return uint32_t but never
  produce `SUCCESS_PRED` — including it is harmless.
- **Reuse of `dsa::fill_*` from `batch.hpp`**: The fill functions already exist and
  match exactly what each operation's `start()` does manually.
- **No `friend` needed**: Since `start(this auto &self)` and `notify(this auto &self)`
  receive `self` as a parameter, they access derived members directly. Derived classes
  use `struct` (all public) — these are internal implementation types, not public API.
