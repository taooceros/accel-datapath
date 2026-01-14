#pragma once
#ifndef DSA_TASK_QUEUE_HPP
#define DSA_TASK_QUEUE_HPP

#include <array>
#include <atomic>
#include <concepts>
#include <cstddef>
#include <mutex>

#include <dsa_stdexec/operation_base.hpp>

namespace dsa {

// Concept for task queue implementations
template <typename T>
concept TaskQueue = requires(T queue, dsa_stdexec::OperationBase *op) {
  // Add an operation to the queue
  { queue.push(op) } -> std::same_as<void>;

  // Poll for completed operations and invoke their callbacks
  // Returns the number of operations that completed
  { queue.poll() } -> std::same_as<std::size_t>;

  // Check if the queue is empty
  { queue.empty() } -> std::same_as<bool>;
};

// Concept for lock implementations
template <typename T>
concept Lockable = requires(T lock) {
  { lock.lock() } -> std::same_as<void>;
  { lock.unlock() } -> std::same_as<void>;
};

namespace locks {

// No-op lock for single-threaded use
class NullLock {
public:
  void lock() {}
  void unlock() {}
};

// Wrapper around std::mutex
class MutexLock {
public:
  void lock() { mutex_.lock(); }
  void unlock() { mutex_.unlock(); }

private:
  std::mutex mutex_;
};

// Simple test-and-set spinlock
class TasSpinlock {
public:
  void lock() {
    while (locked_.test_and_set(std::memory_order_acquire)) {
      __builtin_ia32_pause();
    }
  }

  void unlock() { locked_.clear(std::memory_order_release); }

private:
  std::atomic_flag locked_ = ATOMIC_FLAG_INIT;
};

// Test-and-test-and-set spinlock (TTAS) - more cache-friendly
class TtasSpinlock {
public:
  void lock() {
    for (;;) {
      // First, spin on read (test) - cache-friendly
      while (locked_.load(std::memory_order_relaxed)) {
        __builtin_ia32_pause();
      }
      // Then try to acquire (test-and-set)
      if (!locked_.exchange(true, std::memory_order_acquire)) {
        return; // Successfully acquired
      }
    }
  }

  void unlock() { locked_.store(false, std::memory_order_release); }

private:
  std::atomic<bool> locked_{false};
};

// TTAS spinlock with exponential backoff
class TtasBackoffSpinlock {
public:
  void lock() {
    int backoff = 1;
    constexpr int max_backoff = 1024;

    for (;;) {
      // Test phase - spin on read
      while (locked_.load(std::memory_order_relaxed)) {
        for (int i = 0; i < backoff; ++i) {
          __builtin_ia32_pause();
        }
        if (backoff < max_backoff) {
          backoff *= 2;
        }
      }
      // Test-and-set phase
      if (!locked_.exchange(true, std::memory_order_acquire)) {
        return;
      }
      // Failed, reset backoff and retry
      backoff = 1;
    }
  }

  void unlock() { locked_.store(false, std::memory_order_release); }

private:
  std::atomic<bool> locked_{false};
};

} // namespace locks

// Generic locked task queue - parameterized by lock type and hardware context type
// HwCtx must satisfy the HwContext concept (submit and check_completion methods)
template <Lockable Lock, dsa_stdexec::HwContext HwCtx>
class LockedTaskQueue {
public:
  explicit LockedTaskQueue(HwCtx hw_ctx) : hw_ctx_(std::move(hw_ctx)) {}
  ~LockedTaskQueue() = default;

  LockedTaskQueue(const LockedTaskQueue &) = delete;
  LockedTaskQueue &operator=(const LockedTaskQueue &) = delete;
  LockedTaskQueue(LockedTaskQueue &&) = delete;
  LockedTaskQueue &operator=(LockedTaskQueue &&) = delete;

  void push(dsa_stdexec::OperationBase *op) {
    lock_.lock();
    op->next = head_;
    op->submitted = false;
    head_ = op;
    lock_.unlock();
  }

  std::size_t poll() {
    dsa_stdexec::OperationBase *completed_head = nullptr;

    {
      lock_.lock();
      dsa_stdexec::OperationBase **pprev = &head_;
      dsa_stdexec::OperationBase *curr = head_;

      while (curr != nullptr) {
        // First, submit to hardware if not yet submitted
        if (!curr->submitted) {
          dsa_hw_desc *desc = curr->proxy->get_descriptor();
          if (desc != nullptr) {
            if (hw_ctx_.submit(desc)) {
              curr->submitted = true;
            }
            // If submission failed, we'll retry next poll
          } else {
            // No descriptor (e.g., schedule operation) - mark as submitted
            curr->submitted = true;
          }
        }

        // Check for completion (only if submitted) - static dispatch via HwContext
        if (curr->submitted && hw_ctx_.check_completion(curr)) {
          *pprev = curr->next;
          curr->next = completed_head;
          completed_head = curr;
          curr = *pprev;
        } else {
          pprev = &curr->next;
          curr = curr->next;
        }
      }
      lock_.unlock();
    }

    // Notify outside the lock
    std::size_t count = 0;
    while (completed_head != nullptr) {
      dsa_stdexec::OperationBase *op = completed_head;
      completed_head = op->next;
      op->proxy->notify();
      ++count;
    }

    return count;
  }

  bool empty() {
    lock_.lock();
    bool result = head_ == nullptr;
    lock_.unlock();
    return result;
  }

  HwCtx &hw_context() { return hw_ctx_; }
  const HwCtx &hw_context() const { return hw_ctx_; }

private:
  dsa_stdexec::OperationBase *head_ = nullptr;
  Lock lock_;
  [[no_unique_address]] HwCtx hw_ctx_;
};

// Template aliases for different lock strategies
template <typename HwCtx>
using SingleThreadTaskQueue = LockedTaskQueue<locks::NullLock, HwCtx>;

template <typename HwCtx>
using MutexTaskQueue = LockedTaskQueue<locks::MutexLock, HwCtx>;

template <typename HwCtx>
using TasSpinlockTaskQueue = LockedTaskQueue<locks::TasSpinlock, HwCtx>;

template <typename HwCtx>
using SpinlockTaskQueue = LockedTaskQueue<locks::TtasSpinlock, HwCtx>;

template <typename HwCtx>
using BackoffSpinlockTaskQueue = LockedTaskQueue<locks::TtasBackoffSpinlock, HwCtx>;

// Ring buffer task queue (MPSC - multi-producer, single-consumer)
// Fixed capacity, uses atomic indices for lock-free operation
// Push can be called from multiple threads, poll from single thread
// Maintains a pending list for incomplete operations to avoid re-pushing
template <dsa_stdexec::HwContext HwCtx, std::size_t Capacity = 1024>
class RingBufferTaskQueue {
  static_assert((Capacity & (Capacity - 1)) == 0, "Capacity must be power of 2");

public:
  explicit RingBufferTaskQueue(HwCtx hw_ctx) : hw_ctx_(std::move(hw_ctx)) {
    for (std::size_t i = 0; i < Capacity; ++i) {
      sequence_[i].store(i, std::memory_order_relaxed);
    }
  }
  ~RingBufferTaskQueue() = default;

  RingBufferTaskQueue(const RingBufferTaskQueue &) = delete;
  RingBufferTaskQueue &operator=(const RingBufferTaskQueue &) = delete;
  RingBufferTaskQueue(RingBufferTaskQueue &&) = delete;
  RingBufferTaskQueue &operator=(RingBufferTaskQueue &&) = delete;

  void push(dsa_stdexec::OperationBase *op) {
    op->submitted = false;
    // Reserve a slot using fetch_add
    std::size_t slot = tail_.fetch_add(1, std::memory_order_relaxed);
    std::size_t index = slot & (Capacity - 1);

    // Spin until slot is available (previous consumer has read it)
    // Each slot has a sequence number to track availability
    std::size_t expected_seq = slot;
    while (sequence_[index].load(std::memory_order_acquire) != expected_seq) {
      __builtin_ia32_pause();
    }

    // Write the operation
    buffer_[index] = op;

    // Mark slot as filled by advancing sequence
    sequence_[index].store(slot + 1, std::memory_order_release);
  }

  std::size_t poll() {
    std::size_t count = 0;

    // First, check pending list from previous polls
    dsa_stdexec::OperationBase **pprev = &pending_head_;
    dsa_stdexec::OperationBase *curr = pending_head_;
    dsa_stdexec::OperationBase *completed_head = nullptr;

    while (curr != nullptr) {
      // Submit to hardware if not yet submitted
      if (!curr->submitted) {
        dsa_hw_desc *desc = curr->proxy->get_descriptor();
        if (desc != nullptr) {
          if (hw_ctx_.submit(desc)) {
            curr->submitted = true;
          }
        } else {
          curr->submitted = true;
        }
      }

      // Check for completion - static dispatch via HwContext
      if (curr->submitted && hw_ctx_.check_completion(curr)) {
        // Remove from pending list
        *pprev = curr->next;
        // Add to completed list
        dsa_stdexec::OperationBase *completed = curr;
        curr = curr->next;
        completed->next = completed_head;
        completed_head = completed;
      } else {
        pprev = &curr->next;
        curr = curr->next;
      }
    }

    // Batch consume from ring buffer - find how many items are available
    std::size_t head = head_.load(std::memory_order_relaxed);
    std::size_t available = 0;

    // Count available items
    while (true) {
      std::size_t check_pos = head + available;
      std::size_t index = check_pos & (Capacity - 1);
      std::size_t seq = sequence_[index].load(std::memory_order_acquire);
      if (seq != check_pos + 1) {
        break;
      }
      ++available;
      if (available >= Capacity) break; // Safety limit
    }

    // Batch consume all available items
    for (std::size_t i = 0; i < available; ++i) {
      std::size_t pos = head + i;
      std::size_t index = pos & (Capacity - 1);

      dsa_stdexec::OperationBase *op = buffer_[index];

      // Submit to hardware if not yet submitted
      if (!op->submitted) {
        dsa_hw_desc *desc = op->proxy->get_descriptor();
        if (desc != nullptr) {
          if (hw_ctx_.submit(desc)) {
            op->submitted = true;
          }
        } else {
          op->submitted = true;
        }
      }

      // Check completion immediately (only if submitted) - static dispatch via HwContext
      if (op->submitted && hw_ctx_.check_completion(op)) {
        op->next = completed_head;
        completed_head = op;
      } else {
        // Add to pending list (prepend)
        op->next = pending_head_;
        pending_head_ = op;
      }
    }

    // Batch advance head and release slots
    for (std::size_t i = 0; i < available; ++i) {
      std::size_t pos = head + i;
      std::size_t index = pos & (Capacity - 1);
      sequence_[index].store(pos + Capacity, std::memory_order_release);
    }

    if (available > 0) {
      head_.store(head + available, std::memory_order_relaxed);
    }

    // Notify all completed operations
    while (completed_head != nullptr) {
      dsa_stdexec::OperationBase *op = completed_head;
      completed_head = op->next;
      op->proxy->notify();
      ++count;
    }

    return count;
  }

  bool empty() const {
    if (pending_head_ != nullptr) {
      return false;
    }
    std::size_t head = head_.load(std::memory_order_relaxed);
    std::size_t index = head & (Capacity - 1);
    std::size_t seq = sequence_[index].load(std::memory_order_acquire);
    return seq != head + 1;
  }

  HwCtx &hw_context() { return hw_ctx_; }
  const HwCtx &hw_context() const { return hw_ctx_; }

private:
  alignas(64) std::atomic<std::size_t> head_{0};
  alignas(64) std::atomic<std::size_t> tail_{0};
  std::array<dsa_stdexec::OperationBase *, Capacity> buffer_{};
  std::array<std::atomic<std::size_t>, Capacity> sequence_{};
  // Pending list for operations not yet complete (consumer-only, no sync needed)
  dsa_stdexec::OperationBase *pending_head_{nullptr};
  [[no_unique_address]] HwCtx hw_ctx_;
};

// Lock-free task queue using atomic operations
// Supports concurrent push from multiple threads
// Poll should be called from a single thread (or externally synchronized)
template <dsa_stdexec::HwContext HwCtx>
class LockFreeTaskQueue {
public:
  explicit LockFreeTaskQueue(HwCtx hw_ctx) : hw_ctx_(std::move(hw_ctx)) {}
  ~LockFreeTaskQueue() = default;

  LockFreeTaskQueue(const LockFreeTaskQueue &) = delete;
  LockFreeTaskQueue &operator=(const LockFreeTaskQueue &) = delete;
  LockFreeTaskQueue(LockFreeTaskQueue &&) = delete;
  LockFreeTaskQueue &operator=(LockFreeTaskQueue &&) = delete;

  void push(dsa_stdexec::OperationBase *op) {
    op->submitted = false;
    dsa_stdexec::OperationBase *old_head =
        head_.load(std::memory_order_relaxed);
    do {
      op->next = old_head;
    } while (!head_.compare_exchange_weak(old_head, op,
                                          std::memory_order_release,
                                          std::memory_order_relaxed));
  }

  std::size_t poll() {
    // Atomically steal the entire list
    dsa_stdexec::OperationBase *local_head =
        head_.exchange(nullptr, std::memory_order_acquire);

    if (local_head == nullptr) {
      return 0;
    }

    // Reverse the list to process in FIFO order
    dsa_stdexec::OperationBase *reversed = nullptr;
    while (local_head != nullptr) {
      dsa_stdexec::OperationBase *next = local_head->next;
      local_head->next = reversed;
      reversed = local_head;
      local_head = next;
    }

    // Check completions and separate completed from pending
    dsa_stdexec::OperationBase *completed_head = nullptr;
    dsa_stdexec::OperationBase *pending_head = nullptr;

    while (reversed != nullptr) {
      dsa_stdexec::OperationBase *op = reversed;
      reversed = op->next;

      // Submit to hardware if not yet submitted
      if (!op->submitted) {
        dsa_hw_desc *desc = op->proxy->get_descriptor();
        if (desc != nullptr) {
          if (hw_ctx_.submit(desc)) {
            op->submitted = true;
          }
        } else {
          op->submitted = true;
        }
      }

      // Check for completion - static dispatch via HwContext
      if (op->submitted && hw_ctx_.check_completion(op)) {
        op->next = completed_head;
        completed_head = op;
      } else {
        op->next = pending_head;
        pending_head = op;
      }
    }

    // Re-add pending operations back to the queue
    while (pending_head != nullptr) {
      dsa_stdexec::OperationBase *op = pending_head;
      pending_head = op->next;
      // Don't reset submitted flag when re-adding
      dsa_stdexec::OperationBase *old_head =
          head_.load(std::memory_order_relaxed);
      do {
        op->next = old_head;
      } while (!head_.compare_exchange_weak(old_head, op,
                                            std::memory_order_release,
                                            std::memory_order_relaxed));
    }

    // Notify completed operations
    std::size_t count = 0;
    while (completed_head != nullptr) {
      dsa_stdexec::OperationBase *op = completed_head;
      completed_head = op->next;
      op->proxy->notify();
      ++count;
    }

    return count;
  }

  bool empty() const { return head_.load(std::memory_order_relaxed) == nullptr; }

  HwCtx &hw_context() { return hw_ctx_; }
  const HwCtx &hw_context() const { return hw_ctx_; }

private:
  std::atomic<dsa_stdexec::OperationBase *> head_{nullptr};
  [[no_unique_address]] HwCtx hw_ctx_;
};

} // namespace dsa

#endif
