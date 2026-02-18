#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN
#include <doctest/doctest.h>

#include <algorithm>
#include <atomic>
#include <barrier>
#include <chrono>
#include <thread>
#include <vector>

#include <dsa/mock_dsa.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/operation_base.hpp>

// Helper: wraps a MockOperation in an OperationBase for task queue testing.
// Inherits from OperationBase so function pointers can recover the concrete type.
struct TestOpWrapper : dsa_stdexec::OperationBase {
  MockOperation mock;

  TestOpWrapper() {
    notify_fn = [](dsa_stdexec::OperationBase *base) {
      static_cast<TestOpWrapper *>(base)->mock.notify();
    };
    get_descriptor_fn = [](dsa_stdexec::OperationBase *base) {
      return static_cast<TestOpWrapper *>(base)->mock.get_descriptor();
    };
  }

  TestOpWrapper(const TestOpWrapper &) = delete;
  TestOpWrapper &operator=(const TestOpWrapper &) = delete;
  TestOpWrapper(TestOpWrapper &&) = delete;
  TestOpWrapper &operator=(TestOpWrapper &&) = delete;
};

// Namespace aliases for convenience
using dsa::SingleThreadTaskQueue;
using dsa::MutexTaskQueue;
using dsa::TasSpinlockTaskQueue;
using dsa::SpinlockTaskQueue;
using dsa::BackoffSpinlockTaskQueue;
using dsa::LockFreeTaskQueue;

// ============================================================================
// Basic push/poll for each queue type
// ============================================================================

TEST_SUITE("TaskQueue") {

TEST_CASE("SingleThreadTaskQueue - basic push and poll") {
  MockHwContext hw;
  SingleThreadTaskQueue<MockHwContext> queue(hw);

  REQUIRE(queue.empty());

  TestOpWrapper op;
  queue.push(&op);

  REQUIRE_FALSE(queue.empty());
  REQUIRE_FALSE(op.mock.was_notified());

  std::size_t completed = queue.poll();

  CHECK(completed == 1);
  CHECK(op.mock.was_notified());
  CHECK(queue.empty());
}

TEST_CASE("MutexTaskQueue - basic push and poll") {
  MockHwContext hw;
  MutexTaskQueue<MockHwContext> queue(hw);

  REQUIRE(queue.empty());

  TestOpWrapper op;
  queue.push(&op);

  REQUIRE_FALSE(queue.empty());
  std::size_t completed = queue.poll();

  CHECK(completed == 1);
  CHECK(op.mock.was_notified());
  CHECK(queue.empty());
}

TEST_CASE("TasSpinlockTaskQueue - basic push and poll") {
  MockHwContext hw;
  TasSpinlockTaskQueue<MockHwContext> queue(hw);

  TestOpWrapper op;
  queue.push(&op);
  CHECK(queue.poll() == 1);
  CHECK(op.mock.was_notified());
}

TEST_CASE("SpinlockTaskQueue (TTAS) - basic push and poll") {
  MockHwContext hw;
  SpinlockTaskQueue<MockHwContext> queue(hw);

  TestOpWrapper op;
  queue.push(&op);
  CHECK(queue.poll() == 1);
  CHECK(op.mock.was_notified());
}

TEST_CASE("BackoffSpinlockTaskQueue - basic push and poll") {
  MockHwContext hw;
  BackoffSpinlockTaskQueue<MockHwContext> queue(hw);

  TestOpWrapper op;
  queue.push(&op);
  CHECK(queue.poll() == 1);
  CHECK(op.mock.was_notified());
}

TEST_CASE("RingBufferTaskQueue - basic push and poll") {
  MockHwContext hw;
  dsa::RingBufferTaskQueue<MockHwContext, 16> queue(hw);

  REQUIRE(queue.empty());

  TestOpWrapper op;
  queue.push(&op);

  REQUIRE_FALSE(queue.empty());
  CHECK(queue.poll() == 1);
  CHECK(op.mock.was_notified());
  CHECK(queue.empty());
}

TEST_CASE("LockFreeTaskQueue - basic push and poll") {
  MockHwContext hw;
  LockFreeTaskQueue<MockHwContext> queue(hw);

  REQUIRE(queue.empty());

  TestOpWrapper op;
  queue.push(&op);

  REQUIRE_FALSE(queue.empty());
  CHECK(queue.poll() == 1);
  CHECK(op.mock.was_notified());
  CHECK(queue.empty());
}

// ============================================================================
// Empty queue behavior
// ============================================================================

TEST_CASE("Empty queue behavior") {
  SUBCASE("SingleThreadTaskQueue") {
    MockHwContext hw;
    SingleThreadTaskQueue<MockHwContext> queue(hw);
    CHECK(queue.empty());
    CHECK(queue.poll() == 0);
    CHECK(queue.empty());
  }
  SUBCASE("MutexTaskQueue") {
    MockHwContext hw;
    MutexTaskQueue<MockHwContext> queue(hw);
    CHECK(queue.empty());
    CHECK(queue.poll() == 0);
  }
  SUBCASE("RingBufferTaskQueue") {
    MockHwContext hw;
    dsa::RingBufferTaskQueue<MockHwContext, 16> queue(hw);
    CHECK(queue.empty());
    CHECK(queue.poll() == 0);
  }
  SUBCASE("LockFreeTaskQueue") {
    MockHwContext hw;
    LockFreeTaskQueue<MockHwContext> queue(hw);
    CHECK(queue.empty());
    CHECK(queue.poll() == 0);
  }
}

// ============================================================================
// Multiple operations — all notified
// ============================================================================

TEST_CASE("Multiple operations - all notified") {
  constexpr std::size_t kNumOps = 10;

  SUBCASE("SingleThreadTaskQueue") {
    MockHwContext hw;
    SingleThreadTaskQueue<MockHwContext> queue(hw);

    std::vector<TestOpWrapper> ops(kNumOps);
    for (auto &op : ops) queue.push(&op);

    CHECK(queue.poll() == kNumOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
    CHECK(queue.empty());
  }
  SUBCASE("MutexTaskQueue") {
    MockHwContext hw;
    MutexTaskQueue<MockHwContext> queue(hw);

    std::vector<TestOpWrapper> ops(kNumOps);
    for (auto &op : ops) queue.push(&op);

    CHECK(queue.poll() == kNumOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }
  SUBCASE("RingBufferTaskQueue") {
    MockHwContext hw;
    dsa::RingBufferTaskQueue<MockHwContext, 64> queue(hw);

    std::vector<TestOpWrapper> ops(kNumOps);
    for (auto &op : ops) queue.push(&op);

    CHECK(queue.poll() == kNumOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }
  SUBCASE("LockFreeTaskQueue") {
    MockHwContext hw;
    LockFreeTaskQueue<MockHwContext> queue(hw);

    std::vector<TestOpWrapper> ops(kNumOps);
    for (auto &op : ops) queue.push(&op);

    CHECK(queue.poll() == kNumOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }
}

// ============================================================================
// Concurrent push from multiple threads
// ============================================================================

TEST_CASE("Concurrent push from multiple threads") {
  constexpr std::size_t kNumThreads = 4;
  constexpr std::size_t kOpsPerThread = 100;
  constexpr std::size_t kTotalOps = kNumThreads * kOpsPerThread;

  SUBCASE("MutexTaskQueue") {
    MockHwContext hw;
    MutexTaskQueue<MockHwContext> queue(hw);
    std::vector<TestOpWrapper> ops(kTotalOps);
    std::vector<std::thread> threads;
    std::barrier sync(kNumThreads);

    for (std::size_t t = 0; t < kNumThreads; ++t) {
      threads.emplace_back([&, t] {
        sync.arrive_and_wait();
        for (std::size_t i = 0; i < kOpsPerThread; ++i)
          queue.push(&ops[t * kOpsPerThread + i]);
      });
    }
    for (auto &th : threads) th.join();

    std::size_t total = 0;
    while (total < kTotalOps) total += queue.poll();
    CHECK(total == kTotalOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }

  SUBCASE("SpinlockTaskQueue") {
    MockHwContext hw;
    SpinlockTaskQueue<MockHwContext> queue(hw);
    std::vector<TestOpWrapper> ops(kTotalOps);
    std::vector<std::thread> threads;
    std::barrier sync(kNumThreads);

    for (std::size_t t = 0; t < kNumThreads; ++t) {
      threads.emplace_back([&, t] {
        sync.arrive_and_wait();
        for (std::size_t i = 0; i < kOpsPerThread; ++i)
          queue.push(&ops[t * kOpsPerThread + i]);
      });
    }
    for (auto &th : threads) th.join();

    std::size_t total = 0;
    while (total < kTotalOps) total += queue.poll();
    CHECK(total == kTotalOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }

  SUBCASE("BackoffSpinlockTaskQueue") {
    MockHwContext hw;
    BackoffSpinlockTaskQueue<MockHwContext> queue(hw);
    std::vector<TestOpWrapper> ops(kTotalOps);
    std::vector<std::thread> threads;
    std::barrier sync(kNumThreads);

    for (std::size_t t = 0; t < kNumThreads; ++t) {
      threads.emplace_back([&, t] {
        sync.arrive_and_wait();
        for (std::size_t i = 0; i < kOpsPerThread; ++i)
          queue.push(&ops[t * kOpsPerThread + i]);
      });
    }
    for (auto &th : threads) th.join();

    std::size_t total = 0;
    while (total < kTotalOps) total += queue.poll();
    CHECK(total == kTotalOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }

  SUBCASE("RingBufferTaskQueue") {
    MockHwContext hw;
    dsa::RingBufferTaskQueue<MockHwContext, 512> queue(hw);
    std::vector<TestOpWrapper> ops(kTotalOps);
    std::vector<std::thread> threads;
    std::barrier sync(kNumThreads);

    for (std::size_t t = 0; t < kNumThreads; ++t) {
      threads.emplace_back([&, t] {
        sync.arrive_and_wait();
        for (std::size_t i = 0; i < kOpsPerThread; ++i)
          queue.push(&ops[t * kOpsPerThread + i]);
      });
    }
    for (auto &th : threads) th.join();

    std::size_t total = 0;
    while (total < kTotalOps) total += queue.poll();
    CHECK(total == kTotalOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }

  SUBCASE("LockFreeTaskQueue") {
    MockHwContext hw;
    LockFreeTaskQueue<MockHwContext> queue(hw);
    std::vector<TestOpWrapper> ops(kTotalOps);
    std::vector<std::thread> threads;
    std::barrier sync(kNumThreads);

    for (std::size_t t = 0; t < kNumThreads; ++t) {
      threads.emplace_back([&, t] {
        sync.arrive_and_wait();
        for (std::size_t i = 0; i < kOpsPerThread; ++i)
          queue.push(&ops[t * kOpsPerThread + i]);
      });
    }
    for (auto &th : threads) th.join();

    std::size_t total = 0;
    while (total < kTotalOps) total += queue.poll();
    CHECK(total == kTotalOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }
}

// ============================================================================
// Concurrent push + poll
// ============================================================================

TEST_CASE("Concurrent push and poll") {
  constexpr std::size_t kNumOps = 1000;
  constexpr std::size_t kNumPushers = 3;

  SUBCASE("MutexTaskQueue") {
    MockHwContext hw;
    MutexTaskQueue<MockHwContext> queue(hw);
    std::vector<TestOpWrapper> ops(kNumOps);
    std::atomic<std::size_t> push_idx{0};
    std::atomic<std::size_t> total_completed{0};

    std::vector<std::thread> pushers;
    for (std::size_t i = 0; i < kNumPushers; ++i) {
      pushers.emplace_back([&] {
        while (true) {
          auto idx = push_idx.fetch_add(1, std::memory_order_relaxed);
          if (idx >= kNumOps) break;
          queue.push(&ops[idx]);
          std::this_thread::sleep_for(std::chrono::microseconds(1));
        }
      });
    }

    std::thread poller([&] {
      while (total_completed.load(std::memory_order_relaxed) < kNumOps) {
        total_completed.fetch_add(queue.poll(), std::memory_order_relaxed);
        std::this_thread::yield();
      }
    });

    for (auto &th : pushers) th.join();
    poller.join();

    CHECK(total_completed.load() == kNumOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }

  SUBCASE("LockFreeTaskQueue") {
    MockHwContext hw;
    LockFreeTaskQueue<MockHwContext> queue(hw);
    std::vector<TestOpWrapper> ops(kNumOps);
    std::atomic<std::size_t> push_idx{0};
    std::atomic<std::size_t> total_completed{0};

    std::vector<std::thread> pushers;
    for (std::size_t i = 0; i < kNumPushers; ++i) {
      pushers.emplace_back([&] {
        while (true) {
          auto idx = push_idx.fetch_add(1, std::memory_order_relaxed);
          if (idx >= kNumOps) break;
          queue.push(&ops[idx]);
          std::this_thread::sleep_for(std::chrono::microseconds(1));
        }
      });
    }

    std::thread poller([&] {
      while (total_completed.load(std::memory_order_relaxed) < kNumOps) {
        total_completed.fetch_add(queue.poll(), std::memory_order_relaxed);
        std::this_thread::yield();
      }
    });

    for (auto &th : pushers) th.join();
    poller.join();

    CHECK(total_completed.load() == kNumOps);
    for (const auto &op : ops) CHECK(op.mock.was_notified());
  }
}

// ============================================================================
// RingBuffer capacity limits
// ============================================================================

TEST_CASE("RingBufferTaskQueue capacity limits") {
  constexpr std::size_t kCapacity = 32;
  MockHwContext hw;
  dsa::RingBufferTaskQueue<MockHwContext, kCapacity> queue(hw);

  std::vector<TestOpWrapper> ops(kCapacity);
  for (std::size_t i = 0; i < kCapacity; ++i)
    queue.push(&ops[i]);

  std::size_t total = 0;
  while (total < kCapacity) total += queue.poll();

  CHECK(total == kCapacity);
  for (const auto &op : ops) CHECK(op.mock.was_notified());
  CHECK(queue.empty());
}

// ============================================================================
// LockFree FIFO ordering (steal-and-reverse)
// ============================================================================

TEST_CASE("LockFreeTaskQueue notifies all operations") {
  constexpr std::size_t kNumOps = 20;
  MockHwContext hw;
  LockFreeTaskQueue<MockHwContext> queue(hw);

  std::vector<TestOpWrapper> ops(kNumOps);
  std::vector<std::size_t> order;
  order.reserve(kNumOps);

  for (std::size_t i = 0; i < kNumOps; ++i)
    ops[i].mock.set_callback([&order, i] { order.push_back(i); });

  for (std::size_t i = 0; i < kNumOps; ++i)
    queue.push(&ops[i]);

  CHECK(queue.poll() == kNumOps);
  CHECK(order.size() == kNumOps);

  // Verify all operations were notified (order is not guaranteed)
  std::vector<std::size_t> sorted_order = order;
  std::sort(sorted_order.begin(), sorted_order.end());
  for (std::size_t i = 0; i < kNumOps; ++i)
    CHECK(sorted_order[i] == i);
}

// ============================================================================
// Repeated push/poll cycles
// ============================================================================

TEST_CASE("Multiple poll cycles") {
  constexpr std::size_t kOpsPerCycle = 5;

  SUBCASE("MutexTaskQueue") {
    MockHwContext hw;
    MutexTaskQueue<MockHwContext> queue(hw);

    for (int cycle = 0; cycle < 3; ++cycle) {
      std::vector<TestOpWrapper> ops(kOpsPerCycle);
      for (auto &op : ops) queue.push(&op);
      CHECK(queue.poll() == kOpsPerCycle);
      for (const auto &op : ops) CHECK(op.mock.was_notified());
      CHECK(queue.empty());
    }
  }
  SUBCASE("LockFreeTaskQueue") {
    MockHwContext hw;
    LockFreeTaskQueue<MockHwContext> queue(hw);

    for (int cycle = 0; cycle < 3; ++cycle) {
      std::vector<TestOpWrapper> ops(kOpsPerCycle);
      for (auto &op : ops) queue.push(&op);
      CHECK(queue.poll() == kOpsPerCycle);
      for (const auto &op : ops) CHECK(op.mock.was_notified());
    }
  }
}

// ============================================================================
// Callback invocation
// ============================================================================

TEST_CASE("Operation callback invocation") {
  MockHwContext hw;
  MutexTaskQueue<MockHwContext> queue(hw);

  TestOpWrapper op;
  std::atomic<bool> callback_invoked{false};
  op.mock.set_callback([&] { callback_invoked.store(true); });

  queue.push(&op);
  queue.poll();

  CHECK(op.mock.was_notified());
  CHECK(callback_invoked.load());
}

// ============================================================================
// Stress test
// ============================================================================

TEST_CASE("Stress test - rapid push/poll cycles") {
  constexpr std::size_t kCycles = 100;
  constexpr std::size_t kOpsPerCycle = 10;

  SUBCASE("MutexTaskQueue") {
    MockHwContext hw;
    MutexTaskQueue<MockHwContext> queue(hw);

    for (std::size_t c = 0; c < kCycles; ++c) {
      std::vector<TestOpWrapper> ops(kOpsPerCycle);
      for (auto &op : ops) queue.push(&op);

      std::size_t total = 0;
      while (total < kOpsPerCycle) total += queue.poll();
      CHECK(total == kOpsPerCycle);
    }
  }
  SUBCASE("LockFreeTaskQueue") {
    MockHwContext hw;
    LockFreeTaskQueue<MockHwContext> queue(hw);

    for (std::size_t c = 0; c < kCycles; ++c) {
      std::vector<TestOpWrapper> ops(kOpsPerCycle);
      for (auto &op : ops) queue.push(&op);

      std::size_t total = 0;
      while (total < kOpsPerCycle) total += queue.poll();
      CHECK(total == kOpsPerCycle);
    }
  }
}

} // TEST_SUITE("TaskQueue")
