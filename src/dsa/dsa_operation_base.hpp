#pragma once
#ifndef DSA_OPERATION_BASE_HPP
#define DSA_OPERATION_BASE_HPP

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
struct DsaOperationBase : dsa_stdexec::OperationBase {
  dsa_hw_desc desc_ __attribute__((aligned(64))) = {};
  dsa_completion_record comp_ __attribute__((aligned(32))) = {};
  bool has_descriptor = true;  // Static dispatch flag for get_descriptor
};

} // namespace dsa

#endif
