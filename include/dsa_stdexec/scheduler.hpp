#pragma once
#ifndef DSA_STDEXEC_SCHEDULER_HPP
#define DSA_STDEXEC_SCHEDULER_HPP

#include <dsa/dsa.hpp>
#include <dsa/dsa_operation_base.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <stdexec/execution.hpp>
#include <utility>

namespace dsa_stdexec {

// ScheduleOperation inherits from DsaOperationBase to work with DsaHwContext.
// It pre-sets comp_.status = 1 so check_completion returns true immediately.
// The desc_ member is unused but required by the base class.
template <class DsaType, class ReceiverId>
class ScheduleOperation : public dsa::DsaOperationBase {
  using Receiver = stdexec::__t<ReceiverId>;

public:
  using operation_state_concept = stdexec::operation_state_t;
  struct Wrapper {
    ScheduleOperation *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return nullptr; }  // No HW descriptor for schedule
  };

  ScheduleOperation(DsaType &dsa, Receiver r) : dsa_(dsa), r_(std::move(r)) {
    // Pre-set completion status so check_completion returns true immediately
    comp_.status = 1;
    // No hardware descriptor for schedule operations
    has_descriptor = false;
    proxy = pro::make_proxy<OperationFacade>(Wrapper{this});
  }

  ScheduleOperation(ScheduleOperation &&other) noexcept
      : dsa::DsaOperationBase(), dsa_(other.dsa_), r_(std::move(other.r_)) {
    comp_.status = 1;
    has_descriptor = false;
    proxy = pro::make_proxy<OperationFacade>(Wrapper{this});
  }

  void start() noexcept {
    try {
      dsa_.submit(this);
    } catch (...) {
      stdexec::set_error(std::move(r_), std::current_exception());
    }
  }

  void notify() { stdexec::set_value(std::move(r_)); }

private:
  DsaType &dsa_;
  Receiver r_;
};

template <class DsaType>
class ScheduleSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  explicit ScheduleSender(DsaType &dsa) : dsa_(dsa) {}

  template <stdexec::receiver Receiver>
  auto connect(Receiver &&r) && {
    return ScheduleOperation<DsaType, stdexec::__id<Receiver>>(
        dsa_, std::forward<Receiver>(r));
  }

  template <stdexec::receiver Receiver>
  auto connect(Receiver &&r) const & {
    return ScheduleOperation<DsaType, stdexec::__id<Receiver>>(
        dsa_, std::forward<Receiver>(r));
  }

private:
  DsaType &dsa_;
};

template <class DsaType>
class DsaScheduler {
public:
  using scheduler_concept = stdexec::scheduler_t;
  explicit DsaScheduler(DsaType &dsa) : dsa_(dsa) {}

  ScheduleSender<DsaType> schedule() const noexcept { return ScheduleSender<DsaType>(dsa_); }

  bool operator==(const DsaScheduler &other) const noexcept {
    return &dsa_ == &other.dsa_;
  }

private:
  DsaType &dsa_;
};

} // namespace dsa_stdexec

#endif
