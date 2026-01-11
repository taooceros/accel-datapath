#pragma once
#ifndef DSA_STDEXEC_OPERATION_BASE_HPP
#define DSA_STDEXEC_OPERATION_BASE_HPP

#include <proxy/proxy.h>

extern "C" {
#include <linux/idxd.h>
}

namespace dsa_stdexec {

PRO_DEF_MEM_DISPATCH(CheckCompletion, check_completion);
PRO_DEF_MEM_DISPATCH(Notify, notify);
PRO_DEF_MEM_DISPATCH(GetDescriptor, get_descriptor);

struct OperationFacade
    : pro::facade_builder::add_convention<CheckCompletion, bool()>
          ::add_convention<Notify, void()>
          ::add_convention<GetDescriptor, dsa_hw_desc *()>
          ::build {};

struct OperationBase {
  pro::proxy<OperationFacade> proxy;
  OperationBase *next = nullptr;
  bool submitted = false;  // Track if operation has been submitted to hardware
};

} // namespace dsa_stdexec

#endif
