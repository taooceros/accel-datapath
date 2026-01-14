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
