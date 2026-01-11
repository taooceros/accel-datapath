#pragma once
#ifndef DSA_STDEXEC_SCHEDULER_HPP
#define DSA_STDEXEC_SCHEDULER_HPP

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <stdexec/execution.hpp>
#include <utility>

namespace dsa_stdexec {

template <class ReceiverId> class ScheduleOperation : public OperationBase {
  using Receiver = stdexec::__t<ReceiverId>;

public:
  using operation_state_concept = stdexec::operation_state_t;
  struct Wrapper {
    ScheduleOperation *op;
    bool check_completion() { return op->check_completion(); }
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return nullptr; }  // No HW descriptor for schedule
  };

  ScheduleOperation(Dsa &dsa, Receiver r) : dsa_(dsa), r_(std::move(r)) {
    this->proxy = pro::make_proxy<OperationFacade>(Wrapper{this});
  }

  ScheduleOperation(ScheduleOperation &&other) noexcept
      : OperationBase(), dsa_(other.dsa_), r_(std::move(other.r_)) {
    this->proxy = pro::make_proxy<OperationFacade>(Wrapper{this});
  }

  void start() noexcept {
    try {
      dsa_.submit(this);
    } catch (...) {
      stdexec::set_error(std::move(r_), std::current_exception());
    }
  }

  bool check_completion() {
    return true; // Always ready to run when polled
  }

  void notify() { stdexec::set_value(std::move(r_)); }

private:
  Dsa &dsa_;
  Receiver r_;
};

class ScheduleSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  explicit ScheduleSender(Dsa &dsa) : dsa_(dsa) {}

  template <stdexec::receiver Receiver>
  auto connect(Receiver &&r) && {
    return ScheduleOperation<stdexec::__id<Receiver>>(
        dsa_, std::forward<Receiver>(r));
  }

  template <stdexec::receiver Receiver>
  auto connect(Receiver &&r) const & {
    return ScheduleOperation<stdexec::__id<Receiver>>(
        dsa_, std::forward<Receiver>(r));
  }

private:
  Dsa &dsa_;
};

class DsaScheduler {
public:
  using scheduler_concept = stdexec::scheduler_t;
  explicit DsaScheduler(Dsa &dsa) : dsa_(dsa) {}

  ScheduleSender schedule() const noexcept { return ScheduleSender(dsa_); }

  bool operator==(const DsaScheduler &other) const noexcept {
    return &dsa_ == &other.dsa_;
  }

private:
  Dsa &dsa_;
};

} // namespace dsa_stdexec

#endif
