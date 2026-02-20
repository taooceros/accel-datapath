// Comprehensive stdexec integration tests for dsa-stdexec project
// Uses doctest and MockDsa for testing without real hardware

#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN
#include <doctest/doctest.h>

#include <dsa/mock_dsa.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/scheduler.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <dsa_stdexec/dsa_facade.hpp>
#include <dsa_stdexec/operations/all.hpp>

#include <cstring>
#include <thread>
#include <vector>

// ============================================================================
// TEST SUITE 1: PollingRunLoop
// ============================================================================

TEST_SUITE("PollingRunLoop") {
  TEST_CASE("basic schedule and run") {
    dsa_stdexec::PollingRunLoop loop([] {});
    auto sched = loop.get_scheduler();
    auto sender = sched.schedule();

    bool executed = false;
    dsa_stdexec::wait_start(
        stdexec::then(std::move(sender), [&] { executed = true; }),
        loop);
    CHECK(executed);
  }

  TEST_CASE("multiple tasks execute in order") {
    dsa_stdexec::PollingRunLoop loop([] {});
    auto sched = loop.get_scheduler();

    std::vector<int> execution_order;

    // Schedule and run first task
    auto s1 = stdexec::then(sched.schedule(), [&] { execution_order.push_back(1); });
    dsa_stdexec::wait_start(std::move(s1), loop);
    loop.reset();

    // Schedule and run second task
    auto s2 = stdexec::then(sched.schedule(), [&] { execution_order.push_back(2); });
    dsa_stdexec::wait_start(std::move(s2), loop);
    loop.reset();

    // Schedule and run third task
    auto s3 = stdexec::then(sched.schedule(), [&] { execution_order.push_back(3); });
    dsa_stdexec::wait_start(std::move(s3), loop);

    REQUIRE(execution_order.size() == 3);
    CHECK(execution_order[0] == 1);
    CHECK(execution_order[1] == 2);
    CHECK(execution_order[2] == 3);
  }

  TEST_CASE("finish() stops the loop") {
    dsa_stdexec::PollingRunLoop loop([] {});
    auto sched = loop.get_scheduler();

    // wait_start calls loop.run() and the receiver calls loop.finish()
    auto sender = stdexec::then(sched.schedule(), [] { return 42; });
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 42);
  }

  TEST_CASE("reset() allows re-running") {
    dsa_stdexec::PollingRunLoop loop([] {});
    auto sched = loop.get_scheduler();

    // Run first sender
    auto s1 = stdexec::then(sched.schedule(), [] { return 42; });
    auto r1 = dsa_stdexec::wait_start(std::move(s1), loop);
    REQUIRE(r1.has_value());
    CHECK(std::get<0>(*r1) == 42);

    loop.reset();

    // Run second sender
    auto s2 = stdexec::then(sched.schedule(), [] { return 99; });
    auto r2 = dsa_stdexec::wait_start(std::move(s2), loop);
    REQUIRE(r2.has_value());
    CHECK(std::get<0>(*r2) == 99);
  }

  TEST_CASE("poll function is called during run") {
    int poll_count = 0;
    dsa_stdexec::PollingRunLoop loop([&] { poll_count++; });
    auto sched = loop.get_scheduler();

    auto sender = stdexec::then(sched.schedule(), [] {});
    dsa_stdexec::wait_start(std::move(sender), loop);

    CHECK(poll_count > 0);
  }

  TEST_CASE("poll function with lambda capture") {
    int external_state = 0;
    dsa_stdexec::PollingRunLoop loop([&] { external_state += 5; });
    auto sched = loop.get_scheduler();

    auto sender = stdexec::then(sched.schedule(), [] {});
    dsa_stdexec::wait_start(std::move(sender), loop);

    // Poll should have been called at least once
    CHECK(external_state >= 5);
  }
}

// ============================================================================
// TEST SUITE 2: DsaScheduler with MockDsa
// ============================================================================

TEST_SUITE("DsaScheduler") {
  TEST_CASE("schedule completes on poll") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    bool completed = false;
    auto sender = stdexec::then(sched.schedule(), [&] { completed = true; });
    dsa_stdexec::wait_start(std::move(sender), loop);
    CHECK(completed);
  }

  TEST_CASE("scheduler equality") {
    MockDsaSingleThread dsa;
    dsa_stdexec::DsaScheduler s1(dsa);
    dsa_stdexec::DsaScheduler s2(dsa);
    CHECK(s1 == s2);

    MockDsaSingleThread dsa2;
    dsa_stdexec::DsaScheduler s3(dsa2);
    CHECK_FALSE(s1 == s3);
  }

  TEST_CASE("multiple schedule operations") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    int count = 0;
    for (int i = 0; i < 5; ++i) {
      auto sender = stdexec::then(sched.schedule(), [&] { count++; });
      dsa_stdexec::wait_start(std::move(sender), loop);
      loop.reset();
    }
    CHECK(count == 5);
  }

  TEST_CASE("schedule returns value") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    auto sender = stdexec::then(sched.schedule(), [] { return 123; });
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 123);
  }

  TEST_CASE("schedule with chained operations") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    auto sender = stdexec::then(sched.schedule(), [] { return 10; })
                | stdexec::then([](int x) { return x * 2; })
                | stdexec::then([](int x) { return x + 5; });
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 25); // (10 * 2) + 5
  }
}

// ============================================================================
// TEST SUITE 3: SyncWait Helpers
// ============================================================================

TEST_SUITE("SyncWait") {
  TEST_CASE("sync_wait_threaded with background poller") {
    MockDsa dsa(true);  // start_poller=true for background thread
    dsa_stdexec::DsaScheduler sched(dsa);

    bool completed = false;
    auto sender = stdexec::then(sched.schedule(), [&] { completed = true; });
    dsa_stdexec::sync_wait_threaded(std::move(sender));
    CHECK(completed);
  }

  TEST_CASE("wait_start with value") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    auto sender = stdexec::then(sched.schedule(), [] { return 42; });
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);
    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 42);
  }

  TEST_CASE("wait_start with void sender") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    auto sender = stdexec::then(sched.schedule(), [] {});
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);
    // void sender result is std::optional<std::tuple<>> — should have value
    REQUIRE(result.has_value());
  }

  TEST_CASE("sync_wait_threaded with multiple values") {
    MockDsa dsa(true);  // background poller
    dsa_stdexec::DsaScheduler sched(dsa);

    std::vector<int> results;
    for (int i = 0; i < 3; ++i) {
      auto sender = stdexec::then(sched.schedule(), [i] { return i * 10; });
      auto result = dsa_stdexec::sync_wait_threaded(std::move(sender));
      REQUIRE(result.has_value());
      results.push_back(std::get<0>(*result));
    }

    REQUIRE(results.size() == 3);
    CHECK(results[0] == 0);
    CHECK(results[1] == 10);
    CHECK(results[2] == 20);
  }

  TEST_CASE("wait_start with complex sender chain") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    auto sender = stdexec::then(sched.schedule(), [] { return 5; })
                | stdexec::then([](int x) { return x * x; })
                | stdexec::then([](int x) { return x + 100; });

    auto result = dsa_stdexec::wait_start(std::move(sender), loop);
    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 125); // 5*5 + 100
  }
}

// ============================================================================
// TEST SUITE 4: Operation Senders with MockDsa
// ============================================================================

TEST_SUITE("OperationSenders") {
  TEST_CASE("data_move sender connects and completes") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char src[] = "hello";
    char dst[6] = {};
    auto sender = dsa_stdexec::dsa_data_move(dsa, src, dst, sizeof(src));
    dsa_stdexec::wait_start(std::move(sender), loop);

    // Note: MockDsa doesn't actually copy data, it just completes the operation
    // The test verifies the sender/receiver plumbing works
  }

  TEST_CASE("compare sender returns bool result") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char buf1[] = "test";
    char buf2[] = "test";
    auto sender = dsa_stdexec::dsa_compare(dsa, buf1, buf2, 4);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);
    REQUIRE(result.has_value());
    // MockHwContext completes with status=DSA_COMP_SUCCESS, result field=0
    CHECK(std::get<0>(*result) == true);
  }

  TEST_CASE("mem_fill sender completes") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char buffer[64] = {};
    uint64_t pattern = 0xDEADBEEFCAFEBABE;
    auto sender = dsa_stdexec::dsa_mem_fill(dsa, buffer, sizeof(buffer), pattern);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    // void result - just check it completed
    REQUIRE(result.has_value());
  }

  TEST_CASE("compare_value sender returns bool result") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    uint64_t buffer[] = {0x1234567890ABCDEF, 0x1234567890ABCDEF};
    uint64_t pattern = 0x1234567890ABCDEF;
    auto sender = dsa_stdexec::dsa_compare_value(dsa, buffer, sizeof(buffer), pattern);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == true);
  }

  TEST_CASE("dualcast sender completes") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    alignas(4096) char src[64] = "data";
    alignas(4096) char dst1[64] = {};
    alignas(4096) char dst2[64] = {};
    auto sender = dsa_stdexec::dsa_dualcast(dsa, src, dst1, dst2, 64);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    // void result
    REQUIRE(result.has_value());
  }

  TEST_CASE("crc_gen sender returns uint32_t result") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char data[] = "test data for CRC";
    auto sender = dsa_stdexec::dsa_crc_gen(dsa, data, sizeof(data));
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    // MockHwContext returns 0 for CRC (zeroed completion record)
    CHECK(std::get<0>(*result) == 0);
  }

  TEST_CASE("crc_gen sender with seed") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char data[] = "test";
    uint32_t seed = 0xFFFFFFFF;
    auto sender = dsa_stdexec::dsa_crc_gen(dsa, data, sizeof(data), seed);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 0);
  }

  TEST_CASE("copy_crc sender returns uint32_t result") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char src[] = "data to copy";
    char dst[32] = {};
    auto sender = dsa_stdexec::dsa_copy_crc(dsa, src, dst, sizeof(src));
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 0);
  }

  TEST_CASE("cache_flush sender completes") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char buffer[256] = {};
    auto sender = dsa_stdexec::dsa_cache_flush(dsa, buffer, sizeof(buffer));
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    // void result
    REQUIRE(result.has_value());
  }

  TEST_CASE("chained operations") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char src[64] = "hello";
    char dst[64] = {};

    // Chain data_move with a transform
    auto sender = dsa_stdexec::dsa_data_move(dsa, src, dst, 64)
                | stdexec::then([] { return 42; });

    auto result = dsa_stdexec::wait_start(std::move(sender), loop);
    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 42);
  }

  TEST_CASE("multiple operations sequentially") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char buf1[32] = "test1";
    char buf2[32] = {};
    char buf3[32] = {};

    // First operation
    auto s1 = dsa_stdexec::dsa_data_move(dsa, buf1, buf2, 32);
    dsa_stdexec::wait_start(std::move(s1), loop);
    loop.reset();

    // Second operation
    auto s2 = dsa_stdexec::dsa_data_move(dsa, buf2, buf3, 32);
    dsa_stdexec::wait_start(std::move(s2), loop);
  }

  TEST_CASE("compare with different buffers") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char buf1[] = "data1";
    char buf2[] = "data2";
    auto sender = dsa_stdexec::dsa_compare(dsa, buf1, buf2, 5);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    // MockDsa always returns result=0 (success/equal), so it returns true
    CHECK(std::get<0>(*result) == true);
  }
}

// ============================================================================
// TEST SUITE 5: DsaProxy Type Erasure
// ============================================================================

TEST_SUITE("DsaProxy") {
  // Adapter to make MockDsaSingleThread compatible with DsaFacade
  struct MockDsaAdapter {
    MockDsaSingleThread dsa;

    MockDsaAdapter() : dsa(false) {}

    void submit(dsa_stdexec::OperationBase* op, dsa_hw_desc*) { dsa.submit(op); }
    void submit(dsa_stdexec::OperationBase* op) { dsa.submit(op); }
    void poll() { dsa.poll(); }
    void flush() {} // no-op for mock
  };

  TEST_CASE("DsaProxy wraps adapter and dispatches") {
    MockDsaAdapter adapter;
    auto proxy = dsa_stdexec::make_dsa_proxy<MockDsaAdapter>();
    CHECK(static_cast<bool>(proxy));

    // Can call proxy.poll() etc.
    proxy.poll(); // Should not crash
  }

  TEST_CASE("default DsaProxy is empty") {
    dsa_stdexec::DsaProxy proxy;
    CHECK_FALSE(static_cast<bool>(proxy));
  }

  TEST_CASE("DsaProxy can be moved") {
    auto proxy1 = dsa_stdexec::make_dsa_proxy<MockDsaAdapter>();
    CHECK(static_cast<bool>(proxy1));

    dsa_stdexec::DsaProxy proxy2 = std::move(proxy1);
    CHECK(static_cast<bool>(proxy2));

    // Can call methods on moved-to proxy
    proxy2.poll();
  }
}

// ============================================================================
// TEST SUITE 6: Integration Tests
// ============================================================================

TEST_SUITE("Integration") {
  TEST_CASE("combine scheduler and operations") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    char src[] = "integration test";
    char dst[32] = {};

    // Schedule some work, then do a DSA operation
    auto sender = stdexec::then(sched.schedule(), [] { return 1; })
                | stdexec::then([](int x) { return x + 1; })
                | stdexec::let_value([&](int) {
                    return dsa_stdexec::dsa_data_move(dsa, src, dst, sizeof(src));
                  });

    dsa_stdexec::wait_start(std::move(sender), loop);
  }

  TEST_CASE("multiple DSA operations with scheduler") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    char buf[64] = "data";

    int exec_count = 0;

    // Multiple operations interleaved with scheduler tasks
    for (int i = 0; i < 3; ++i) {
      auto sender = stdexec::then(sched.schedule(), [&] { exec_count++; })
                  | stdexec::let_value([&] {
                      return dsa_stdexec::dsa_cache_flush(dsa, buf, sizeof(buf));
                    });

      dsa_stdexec::wait_start(std::move(sender), loop);
      loop.reset();
    }

    CHECK(exec_count == 3);
  }

  TEST_CASE("compare operation in chain") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
    dsa_stdexec::DsaScheduler sched(dsa);

    char buf1[] = "test";
    char buf2[] = "test";

    auto sender = sched.schedule()
                | stdexec::let_value([&] {
                    return dsa_stdexec::dsa_compare(dsa, buf1, buf2, 4);
                  })
                | stdexec::then([](bool match) { return match ? 100 : 0; });

    auto result = dsa_stdexec::wait_start(std::move(sender), loop);
    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 100);
  }

  TEST_CASE("threaded sync_wait with operations") {
    MockDsa dsa(true); // background poller
    dsa_stdexec::DsaScheduler sched(dsa);

    char data[] = "threaded test";
    auto sender = sched.schedule()
                | stdexec::let_value([&] {
                    return dsa_stdexec::dsa_crc_gen(dsa, data, sizeof(data));
                  });

    auto result = dsa_stdexec::sync_wait_threaded(std::move(sender));
    REQUIRE(result.has_value());
    // CRC result is 0 from mock
    CHECK(std::get<0>(*result) == 0);
  }

  TEST_CASE("stress test - many operations") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char buffers[10][64];
    for (int i = 0; i < 10; ++i) {
      std::snprintf(buffers[i], 64, "buffer%d", i);
    }

    // Submit many operations
    for (int i = 0; i < 9; ++i) {
      auto sender = dsa_stdexec::dsa_data_move(dsa, buffers[i], buffers[i+1], 64);
      dsa_stdexec::wait_start(std::move(sender), loop);
      loop.reset();
    }
  }
}

// ============================================================================
// TEST SUITE 7: Error Handling and Edge Cases
// ============================================================================

TEST_SUITE("EdgeCases") {
  TEST_CASE("empty data_move operation") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char src[1] = {};
    char dst[1] = {};
    auto sender = dsa_stdexec::dsa_data_move(dsa, src, dst, 0);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    // Should complete successfully even with size=0
    REQUIRE(result.has_value());
  }

  TEST_CASE("CRC with zero-length data") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char data[1] = {};
    auto sender = dsa_stdexec::dsa_crc_gen(dsa, data, 0);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == 0);
  }

  TEST_CASE("multiple resets of run loop") {
    dsa_stdexec::PollingRunLoop loop([] {});
    auto sched = loop.get_scheduler();

    for (int i = 0; i < 5; ++i) {
      auto sender = stdexec::then(sched.schedule(), [i] { return i; });
      auto result = dsa_stdexec::wait_start(std::move(sender), loop);
      REQUIRE(result.has_value());
      CHECK(std::get<0>(*result) == i);
      loop.reset();
    }
  }

  TEST_CASE("compare with same buffer") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    char buf[] = "same";
    auto sender = dsa_stdexec::dsa_compare(dsa, buf, buf, sizeof(buf));
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == true);
  }

  TEST_CASE("compare_value with matching pattern") {
    MockDsaSingleThread dsa;
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    uint64_t value = 0xAAAAAAAAAAAAAAAA;
    uint64_t pattern = 0xAAAAAAAAAAAAAAAA;
    auto sender = dsa_stdexec::dsa_compare_value(dsa, &value, sizeof(value), pattern);
    auto result = dsa_stdexec::wait_start(std::move(sender), loop);

    REQUIRE(result.has_value());
    CHECK(std::get<0>(*result) == true);
  }
}

// ============================================================================
// TEST SUITE 8: Concurrency (Limited)
// ============================================================================

TEST_SUITE("Concurrency") {
  TEST_CASE("background poller handles multiple operations") {
    MockDsa dsa(true); // background poller
    dsa_stdexec::DsaScheduler sched(dsa);

    std::vector<int> results;
    for (int i = 0; i < 5; ++i) {
      auto sender = stdexec::then(sched.schedule(), [i] { return i * 2; });
      auto result = dsa_stdexec::sync_wait_threaded(std::move(sender));
      REQUIRE(result.has_value());
      results.push_back(std::get<0>(*result));
    }

    REQUIRE(results.size() == 5);
    for (int i = 0; i < 5; ++i) {
      CHECK(results[i] == i * 2);
    }
  }

  TEST_CASE("background poller with DSA operations") {
    MockDsa dsa(true); // background poller

    char src[] = "concurrent test";
    char dst[32] = {};

    auto sender = dsa_stdexec::dsa_data_move(dsa, src, dst, sizeof(src));
    dsa_stdexec::sync_wait_threaded(std::move(sender));

    // Just verify completion
  }
}
