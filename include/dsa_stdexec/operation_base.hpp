#pragma once
#ifndef DSA_STDEXEC_OPERATION_BASE_HPP
#define DSA_STDEXEC_OPERATION_BASE_HPP

#include <concepts>
#include <proxy/proxy.h>

extern "C" {
#include <linux/idxd.h>
}

namespace dsa_stdexec {

// Proxy dispatch definitions for operation callbacks
// Note: check_completion is NOT here - it's handled by HwContext with static dispatch
PRO_DEF_MEM_DISPATCH(Notify, notify);
PRO_DEF_MEM_DISPATCH(GetDescriptor, get_descriptor);

struct OperationFacade
    : pro::facade_builder::add_convention<Notify, void()>
          ::add_convention<GetDescriptor, dsa_hw_desc *()>
          ::build {};

struct OperationBase {
  pro::proxy<OperationFacade> proxy;
  OperationBase *next = nullptr;
  bool submitted = false;  // Track if operation has been submitted to hardware
};

// Concept for hardware context types that can submit descriptors and check completion.
// This allows the task queue to be parameterized by different hardware backends.
// check_completion uses static dispatch (no virtual call) for performance.
template <typename T>
concept HwContext = requires(T ctx, dsa_hw_desc *desc, OperationBase *op) {
  // Submit a descriptor to hardware. Returns true if successful, false to retry.
  { ctx.submit(desc) } -> std::same_as<bool>;
  
  // Check if an operation has completed. Uses static dispatch.
  { ctx.check_completion(op) } -> std::same_as<bool>;
};

} // namespace dsa_stdexec

#endif
