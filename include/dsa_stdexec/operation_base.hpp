#pragma once
#ifndef DSA_STDEXEC_OPERATION_BASE_HPP
#define DSA_STDEXEC_OPERATION_BASE_HPP

#include <concepts>

extern "C" {
#include <linux/idxd.h>
}

namespace dsa_stdexec {

struct OperationBase {
  void (*notify_fn)(OperationBase *self) = nullptr;
  dsa_hw_desc *(*get_descriptor_fn)(OperationBase *self) = nullptr;
  OperationBase *next = nullptr;

  void notify() { notify_fn(this); }
  dsa_hw_desc *get_descriptor() { return get_descriptor_fn(this); }
};

// Concept for hardware context types used by task queues.
// Task queues only need to check completion - hardware submission happens in start().
template <typename T>
concept HwContext = requires(T ctx, OperationBase *op) {
  // Check if an operation has completed. Uses static dispatch.
  { ctx.check_completion(op) } -> std::same_as<bool>;
};

} // namespace dsa_stdexec

#endif
