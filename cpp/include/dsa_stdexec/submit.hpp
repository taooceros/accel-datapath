#pragma once
#ifndef DSA_STDEXEC_SUBMIT_HPP
#define DSA_STDEXEC_SUBMIT_HPP

#include <exception>
#include <stdexec/execution.hpp>

namespace dsa_stdexec {

struct DetachedReceiver {
  using receiver_concept = stdexec::receiver_t;
  
  void set_value() && noexcept {}
  
  void set_error(std::exception_ptr) && noexcept {}
  
  void set_stopped() && noexcept {}
  
  auto get_env() const noexcept {
    return stdexec::empty_env{};
  }
};

template <class Sender> auto submit(Sender &&snd) {
  return stdexec::connect(std::forward<Sender>(snd), DetachedReceiver{});
}

} // namespace dsa_stdexec

#endif // DSA_STDEXEC_SUBMIT_HPP
