#pragma once
#ifndef MOCK_DSA_HPP
#define MOCK_DSA_HPP

#include <atomic>
#include <chrono>
#include <cstddef>
#include <functional>
#include <thread>

extern "C" {
#include <linux/idxd.h>
}

#include "task_queue.hpp"
#include <dsa_stdexec/operation_base.hpp>

// Mock hardware submission callable for testing task queues without real DSA hardware
// Simulates immediate successful submission
class MockHwSubmit {
public:
  MockHwSubmit() = default;

  // Always returns true (successful submission)
  bool operator()(dsa_hw_desc *desc) const {
    (void)desc;
    return true;
  }
};

// Mock operation that completes immediately or after a delay
// Used for benchmarking task queue overhead without actual DSA operations
class MockOperation {
public:
  // Default: completes immediately
  MockOperation() : complete_at_(std::chrono::steady_clock::time_point::min()) {
    desc_.opcode = DSA_OPCODE_MEMMOVE;
    completion_.status = DSA_COMP_SUCCESS;
    desc_.completion_addr = reinterpret_cast<uintptr_t>(&completion_);
  }

  // Complete after specified delay from construction
  explicit MockOperation(std::chrono::nanoseconds delay)
      : complete_at_(std::chrono::steady_clock::now() + delay) {
    desc_.opcode = DSA_OPCODE_MEMMOVE;
    completion_.status = 0; // Not complete yet
    desc_.completion_addr = reinterpret_cast<uintptr_t>(&completion_);
  }

  // Complete after specified delay from when submitted
  void set_delay_from_submit(std::chrono::nanoseconds delay) {
    delay_from_submit_ = delay;
    use_submit_delay_ = true;
  }

  bool check_completion() {
    if (use_submit_delay_) {
      if (!submitted_) {
        return false;
      }
      if (std::chrono::steady_clock::now() >= submit_time_ + delay_from_submit_) {
        completion_.status = DSA_COMP_SUCCESS;
        return true;
      }
      return false;
    }
    
    if (complete_at_ == std::chrono::steady_clock::time_point::min()) {
      // Immediate completion
      return true;
    }
    if (std::chrono::steady_clock::now() >= complete_at_) {
      completion_.status = DSA_COMP_SUCCESS;
      return true;
    }
    return false;
  }

  void notify() {
    notified_ = true;
    if (callback_) {
      callback_();
    }
  }

  dsa_hw_desc *get_descriptor() {
    if (!submitted_ && use_submit_delay_) {
      submit_time_ = std::chrono::steady_clock::now();
      submitted_ = true;
    }
    return &desc_;
  }

  bool was_notified() const { return notified_; }

  void set_callback(std::function<void()> cb) { callback_ = std::move(cb); }

private:
  dsa_hw_desc desc_{};
  dsa_completion_record completion_{};
  std::chrono::steady_clock::time_point complete_at_;
  std::chrono::steady_clock::time_point submit_time_;
  std::chrono::nanoseconds delay_from_submit_{0};
  bool use_submit_delay_ = false;
  bool submitted_ = false;
  bool notified_ = false;
  std::function<void()> callback_;
};

// Template aliases for task queues with mock hardware submission
template <template <typename> class QueueTemplate>
using MockTaskQueue = QueueTemplate<MockHwSubmit>;

// Mock DSA class for benchmarking task queues without real hardware
template <template <typename> class QueueTemplate = dsa::MutexTaskQueue>
class MockDsaBase {
public:
  using Queue = MockTaskQueue<QueueTemplate>;

  explicit MockDsaBase(bool start_poller = false)
      : task_queue_(MockHwSubmit{}) {
    if (start_poller) {
      running_.store(true, std::memory_order_relaxed);
      poller_ = std::thread([this] {
        while (running_.load(std::memory_order_relaxed)) {
          poll();
        }
      });
    }
  }

  ~MockDsaBase() {
    running_.store(false, std::memory_order_relaxed);
    if (poller_.joinable()) {
      poller_.join();
    }
  }

  void submit(dsa_stdexec::OperationBase *op) {
    task_queue_.push(op);
  }

  std::size_t poll() {
    return task_queue_.poll();
  }

  Queue &task_queue() noexcept { return task_queue_; }
  const Queue &task_queue() const noexcept { return task_queue_; }

  bool is_polling() const { return running_.load(std::memory_order_relaxed); }

private:
  Queue task_queue_;
  std::thread poller_;
  std::atomic<bool> running_{false};

  MockDsaBase(const MockDsaBase &) = delete;
  MockDsaBase &operator=(const MockDsaBase &) = delete;
};

// Convenience aliases for different queue strategies with mock hardware
using MockDsa = MockDsaBase<dsa::MutexTaskQueue>;
using MockDsaSingleThread = MockDsaBase<dsa::SingleThreadTaskQueue>;
using MockDsaTasSpinlock = MockDsaBase<dsa::TasSpinlockTaskQueue>;
using MockDsaSpinlock = MockDsaBase<dsa::SpinlockTaskQueue>;
using MockDsaBackoffSpinlock = MockDsaBase<dsa::BackoffSpinlockTaskQueue>;
using MockDsaLockFree = MockDsaBase<dsa::LockFreeTaskQueue>;

#endif
