#pragma once
#ifndef DSA_STDEXEC_DSA_FACADE_HPP
#define DSA_STDEXEC_DSA_FACADE_HPP

#include <proxy/proxy.h>
#include <dsa_stdexec/operation_base.hpp>

namespace dsa_stdexec {

// Proxy dispatches for the DSA interface
PRO_DEF_MEM_DISPATCH(DsaSubmit, submit);
PRO_DEF_MEM_DISPATCH(DsaPoll, poll);
PRO_DEF_MEM_DISPATCH(DsaFlush, flush);

// Type-erasing facade for any DSA type (DsaBase<Q> or DsaBatchBase<Q>).
// Handles both submit overloads via multi-overload convention.
// Non-copyable — DSA objects own threads and hardware resources.
struct DsaFacade
    : pro::facade_builder
          ::add_convention<DsaSubmit, void(OperationBase *, dsa_hw_desc *), void(OperationBase *)>
          ::add_convention<DsaPoll, void()>
          ::add_convention<DsaFlush, void()>
          ::support_copy<pro::constraint_level::none>
          ::build {};

// Owning type-erased DSA handle.
// Wraps pro::proxy<DsaFacade> to provide dot-syntax (dsa.submit(), dsa.poll())
// instead of arrow-syntax (dsa->submit()), so operation senders and schedulers
// work without changes.
class DsaProxy {
public:
  DsaProxy() = default;
  DsaProxy(pro::proxy<DsaFacade> p) : p_(std::move(p)) {}

  void submit(OperationBase *op, dsa_hw_desc *desc) { p_->submit(op, desc); }
  void submit(OperationBase *op) { p_->submit(op); }
  void poll() { p_->poll(); }
  void flush() { p_->flush(); }

  explicit operator bool() const noexcept { return p_.has_value(); }

private:
  pro::proxy<DsaFacade> p_;
};

// Factory: construct a DsaProxy owning a heap-allocated T
template <typename T, typename... Args>
DsaProxy make_dsa_proxy(Args &&...args) {
  return DsaProxy(pro::make_proxy<DsaFacade, T>(std::forward<Args>(args)...));
}

} // namespace dsa_stdexec

#endif
