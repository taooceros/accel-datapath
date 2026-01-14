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
struct DsaOperationBase : dsa_stdexec::OperationBase {
  dsa_hw_desc desc_ __attribute__((aligned(64))) = {};
  dsa_completion_record comp_ __attribute__((aligned(32))) = {};
};

} // namespace dsa

#endif
