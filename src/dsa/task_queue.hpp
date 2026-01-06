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

namespace detail {

// Helper to poll and notify completed operations from a linked list
// Returns new head and count of completed operations
// Caller must hold any necessary locks during this call
inline std::size_t poll_and_notify(dsa_stdexec::OperationBase *&head) {
  dsa_stdexec::OperationBase *completed_head = nullptr;
  dsa_stdexec::OperationBase **pprev = &head;
  dsa_stdexec::OperationBase *curr = head;

  while (curr != nullptr) {
    if (curr->proxy->check_completion()) {
      // Remove from list
      *pprev = curr->next;

      // Add to completed list
      curr->next = completed_head;
      completed_head = curr;

      // Move to next (pprev stays the same because we removed curr)
      curr = *pprev;
    } else {
      // Move to next
      pprev = &curr->next;
      curr = curr->next;
    }
  }

  // Notify and count completed operations
  std::size_t count = 0;
  while (completed_head != nullptr) {
    dsa_stdexec::OperationBase *op = completed_head;
    completed_head = op->next;
    op->proxy->notify();
    ++count;
  }

  return count;
}

} // namespace detail

// Generic locked task queue - parameterized by lock type
template <Lockable Lock>
class LockedTaskQueue {
public:
  LockedTaskQueue() = default;
  ~LockedTaskQueue() = default;

  LockedTaskQueue(const LockedTaskQueue &) = delete;
  LockedTaskQueue &operator=(const LockedTaskQueue &) = delete;
  LockedTaskQueue(LockedTaskQueue &&) = delete;
  LockedTaskQueue &operator=(LockedTaskQueue &&) = delete;

  void push(dsa_stdexec::OperationBase *op) {
    lock_.lock();
    op->next = head_;
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
        if (curr->proxy->check_completion()) {
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

private:
  dsa_stdexec::OperationBase *head_ = nullptr;
  Lock lock_;
};

// Single-threaded task queue - no synchronization
using SingleThreadTaskQueue = LockedTaskQueue<locks::NullLock>;

// Mutex-based task queue - safe for multi-threaded access
using MutexTaskQueue = LockedTaskQueue<locks::MutexLock>;

// TAS Spinlock-based task queue
using TasSpinlockTaskQueue = LockedTaskQueue<locks::TasSpinlock>;

// TTAS Spinlock-based task queue - more cache-friendly
using SpinlockTaskQueue = LockedTaskQueue<locks::TtasSpinlock>;

// TTAS Spinlock with backoff
using BackoffSpinlockTaskQueue = LockedTaskQueue<locks::TtasBackoffSpinlock>;

// Ring buffer task queue (MPSC - multi-producer, single-consumer)
// Fixed capacity, uses atomic indices for lock-free operation
// Push can be called from multiple threads, poll from single thread
template <std::size_t Capacity = 1024>
class RingBufferTaskQueue {
  static_assert((Capacity & (Capacity - 1)) == 0, "Capacity must be power of 2");

public:
  RingBufferTaskQueue() {
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
    dsa_stdexec::OperationBase *completed_head = nullptr;
    std::size_t count = 0;

    // Process all available items
    while (true) {
      std::size_t head = head_.load(std::memory_order_relaxed);
      std::size_t index = head & (Capacity - 1);

      // Check if slot has data (sequence should be head + 1)
      std::size_t seq = sequence_[index].load(std::memory_order_acquire);
      if (seq != head + 1) {
        break; // No more items available
      }

      // Read the operation
      dsa_stdexec::OperationBase *op = buffer_[index];

      // Advance head
      head_.store(head + 1, std::memory_order_relaxed);

      // Mark slot as consumed (ready for next round)
      sequence_[index].store(head + Capacity, std::memory_order_release);

      // Check completion
      if (op->proxy->check_completion()) {
        op->next = completed_head;
        completed_head = op;
      } else {
        // Re-queue if not complete
        push(op);
      }
    }

    // Notify completed operations
    while (completed_head != nullptr) {
      dsa_stdexec::OperationBase *op = completed_head;
      completed_head = op->next;
      op->proxy->notify();
      ++count;
    }

    return count;
  }

  bool empty() const {
    std::size_t head = head_.load(std::memory_order_relaxed);
    std::size_t index = head & (Capacity - 1);
    std::size_t seq = sequence_[index].load(std::memory_order_acquire);
    return seq != head + 1;
  }

private:
  alignas(64) std::atomic<std::size_t> head_{0};
  alignas(64) std::atomic<std::size_t> tail_{0};
  std::array<dsa_stdexec::OperationBase *, Capacity> buffer_{};
  std::array<std::atomic<std::size_t>, Capacity> sequence_{};
};

// Convenience alias with default capacity
using RingBufferTaskQueue1K = RingBufferTaskQueue<1024>;

// Lock-free task queue using atomic operations
// Supports concurrent push from multiple threads
// Poll should be called from a single thread (or externally synchronized)
class LockFreeTaskQueue {
public:
  LockFreeTaskQueue() = default;
  ~LockFreeTaskQueue() = default;

  LockFreeTaskQueue(const LockFreeTaskQueue &) = delete;
  LockFreeTaskQueue &operator=(const LockFreeTaskQueue &) = delete;
  LockFreeTaskQueue(LockFreeTaskQueue &&) = delete;
  LockFreeTaskQueue &operator=(LockFreeTaskQueue &&) = delete;

  void push(dsa_stdexec::OperationBase *op) {
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

      if (op->proxy->check_completion()) {
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
      push(op);
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

private:
  std::atomic<dsa_stdexec::OperationBase *> head_{nullptr};
};

// Type alias for default task queue
using DefaultTaskQueue = MutexTaskQueue;

} // namespace dsa

#endif
