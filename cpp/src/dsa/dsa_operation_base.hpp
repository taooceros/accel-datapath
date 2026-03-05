#pragma once
#ifndef DSA_OPERATION_BASE_HPP
#define DSA_OPERATION_BASE_HPP

#include <cstdint>
#include <dsa_stdexec/operation_base.hpp>

extern "C" {
#include <linux/idxd.h>
}

namespace dsa {

// Base class for DSA operations that need hardware descriptor and completion record.
// Inherits from OperationBase to work with task queues.
// Operations like DataMoveOperation inherit from this.
// ScheduleOperation also inherits but pre-sets comp_.status for immediate completion.
//
// The has_descriptor flag enables static dispatch for get_descriptor():
// - true: operation has a hardware descriptor (e.g., DataMoveOperation)
// - false: operation has no descriptor (e.g., ScheduleOperation)
// This avoids virtual dispatch overhead in the hot path.
//
// IMPORTANT: DSA hardware requires:
// - Descriptors to be 64-byte aligned
// - Completion records to be 32-byte aligned
//
// We use over-allocated buffers and compute aligned addresses at runtime because
// coroutine frame allocators don't respect alignas() on types. The alignas(64) on
// the struct may not be honored when allocated within a coroutine frame.
struct alignas(64) DsaOperationBase : dsa_stdexec::OperationBase {
  // Over-allocate: 64 bytes for descriptor + 63 bytes padding for alignment
  alignas(64) char desc_buffer_[64 + 63] = {};
  // Over-allocate: 32 bytes for completion record + 31 bytes padding for alignment
  alignas(32) char comp_buffer_[32 + 31] = {};
  bool has_descriptor = true;  // Static dispatch flag for get_descriptor

  // Cached aligned pointers — computed once at construction to avoid
  // repeated alignment arithmetic in the hot poll path.
  dsa_hw_desc* const desc_cached_;
  dsa_completion_record* const comp_cached_;

  DsaOperationBase()
      : desc_cached_(reinterpret_cast<dsa_hw_desc*>(
            (reinterpret_cast<uintptr_t>(desc_buffer_) + 63) & ~uintptr_t{63})),
        comp_cached_(reinterpret_cast<dsa_completion_record*>(
            (reinterpret_cast<uintptr_t>(comp_buffer_) + 31) & ~uintptr_t{31})) {}

  dsa_hw_desc* desc_ptr() noexcept { return desc_cached_; }
  dsa_completion_record* comp_ptr() noexcept { return comp_cached_; }
  const dsa_completion_record* comp_ptr() const noexcept { return comp_cached_; }
};

} // namespace dsa

#endif
