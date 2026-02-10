#pragma once
#ifndef BENCHMARK_HELPERS_HPP
#define BENCHMARK_HELPERS_HPP

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fmt/format.h>
#include <new>
#include <numeric>
#include <stdexec/execution.hpp>
#include <string>
#include <string_view>
#include <unistd.h>
#include <vector>

// Progress bar with time-based throttling to minimize performance impact
class ProgressBar {
public:
  ProgressBar(size_t total, std::string_view label = "")
      : total_(total), current_(0), label_(label), bar_width_(30) {
    is_tty_ = isatty(STDERR_FILENO);
    last_update_ = std::chrono::steady_clock::now();
  }

  void set_label(std::string_view label) { label_ = label; }

  void update(size_t current) {
    current_ = current;
    auto now = std::chrono::steady_clock::now();
    auto elapsed =
        std::chrono::duration_cast<std::chrono::milliseconds>(now - last_update_);
    // Throttle updates to max ~10Hz to minimize overhead
    if (elapsed.count() >= 100 || current >= total_) {
      render();
      last_update_ = now;
    }
  }

  void increment() { update(current_ + 1); }

  void finish() {
    current_ = total_;
    render();
    if (is_tty_) {
      fmt::print(stderr, "\n");
    }
  }

private:
  void render() {
    if (!is_tty_)
      return;

    double pct = total_ > 0 ? static_cast<double>(current_) / total_ : 0.0;
    size_t filled = static_cast<size_t>(pct * bar_width_);

    std::string bar(filled, '=');
    if (filled < bar_width_) {
      bar += '>';
      bar += std::string(bar_width_ - filled - 1, ' ');
    }

    fmt::print(stderr, "\r\033[K{} [{}] {:3.0f}% ({}/{})", label_, bar, pct * 100,
               current_, total_);
    std::fflush(stderr);
  }

  size_t total_;
  size_t current_;
  std::string label_;
  size_t bar_width_;
  bool is_tty_;
  std::chrono::steady_clock::time_point last_update_;
};

// Latency collector (single-threaded, no locking needed)
class LatencyCollector {
public:
  void record(double latency_ns) { samples_.push_back(latency_ns); }

  void reserve(size_t n) { samples_.reserve(n); }

  void clear() { samples_.clear(); }

  struct Stats {
    double min_ns = 0;
    double max_ns = 0;
    double avg_ns = 0;
    double p50_ns = 0;
    double p99_ns = 0;
    size_t count = 0;
  };

  Stats compute_stats() {
    if (samples_.empty()) {
      return {};
    }

    std::sort(samples_.begin(), samples_.end());
    Stats s;
    s.count = samples_.size();
    s.min_ns = samples_.front();
    s.max_ns = samples_.back();
    s.avg_ns = std::accumulate(samples_.begin(), samples_.end(), 0.0) / s.count;
    s.p50_ns = samples_[s.count / 2];
    s.p99_ns = samples_[static_cast<size_t>(s.count * 0.99)];
    return s;
  }

private:
  std::vector<double> samples_;
};

// Metric result structure
struct DsaMetric {
  double bandwidth;      // GB/s
  double msg_rate;       // Million messages/second
  uint64_t page_faults;
  LatencyCollector::Stats latency;
};

// Benchmark result for one configuration
struct BenchmarkResult {
  size_t concurrency;
  size_t msg_size;
  DsaMetric single_thread;
  DsaMetric mutex;
  DsaMetric tas_spinlock;
  DsaMetric ttas_spinlock;
  DsaMetric backoff_spinlock;
  DsaMetric lockfree;
};

// Buffer set for all operation types
// Manages src/dst buffers plus special allocations for dualcast
struct BufferSet {
  std::vector<char> src;   // source buffer (all ops)
  std::vector<char> dst;   // destination / second source for compare
  char *dualcast_dst1 = nullptr;  // 4KB-aligned for dualcast
  char *dualcast_dst2 = nullptr;  // 4KB-aligned, same bits[11:0] as dst1
  static constexpr uint64_t fill_pattern = 0xDEADBEEFCAFEBABE;

  explicit BufferSet(size_t size)
      : src(size), dst(size) {
    std::memset(src.data(), 1, size);
    // dst initialized same as src so compare operations succeed
    std::memset(dst.data(), 1, size);
    // For compare_value: fill src with the pattern so comparisons succeed
    // (only used when running compare_value benchmark)

    // Dualcast requires both destinations to have same bits[11:0]
    // Allocating at 4KB alignment ensures bits[11:0] are all zero for both
    dualcast_dst1 = static_cast<char *>(std::aligned_alloc(4096, size));
    dualcast_dst2 = static_cast<char *>(std::aligned_alloc(4096, size));
    if (dualcast_dst1) std::memset(dualcast_dst1, 0, size);
    if (dualcast_dst2) std::memset(dualcast_dst2, 0, size);
  }

  ~BufferSet() {
    std::free(dualcast_dst1);
    std::free(dualcast_dst2);
  }

  BufferSet(const BufferSet &) = delete;
  BufferSet &operator=(const BufferSet &) = delete;
};

// Minimal receiver that signals a pre-allocated slot is ready for reuse.
// Used with OperationSlot for zero-allocation sliding window.
struct SlotReceiver {
  using receiver_concept = stdexec::receiver_t;
  std::atomic<bool> *slot_ready;

  void set_value(auto &&...) && noexcept {
    slot_ready->store(true, std::memory_order_release);
  }
  void set_error(auto &&) && noexcept {
    slot_ready->store(true, std::memory_order_release);
  }
  void set_stopped() && noexcept {
    slot_ready->store(true, std::memory_order_release);
  }
  auto get_env() const noexcept { return stdexec::empty_env{}; }
};

// Pre-allocated slot for a single operation state.
// Avoids heap allocation by using placement new into fixed storage.
// StorageSize: 448 for inline paths, 768 for threaded (schedule | let_value).
template <size_t StorageSize = 768> struct OperationSlot {
  alignas(64) char storage[StorageSize];
  std::atomic<bool> ready{true};
  void (*destroy_fn)(void *) = nullptr;

  template <class Sender> void start_op(Sender &&sender) {
    using Op = stdexec::connect_result_t<Sender, SlotReceiver>;
    static_assert(sizeof(Op) <= StorageSize,
                  "Operation state too large for slot; bump StorageSize");
    static_assert(alignof(Op) <= 64);

    if (destroy_fn) {
      destroy_fn(storage);
      destroy_fn = nullptr;
    }
    ready.store(false, std::memory_order_release);
    auto *op = new (storage)
        Op(stdexec::connect(std::forward<Sender>(sender), SlotReceiver{&ready}));
    destroy_fn = [](void *p) { static_cast<Op *>(p)->~Op(); };
    stdexec::start(*op);
  }

  ~OperationSlot() {
    if (destroy_fn)
      destroy_fn(storage);
  }

  OperationSlot() = default;
  OperationSlot(const OperationSlot &) = delete;
  OperationSlot &operator=(const OperationSlot &) = delete;
  OperationSlot(OperationSlot &&) = delete;
  OperationSlot &operator=(OperationSlot &&) = delete;
};

#endif // BENCHMARK_HELPERS_HPP
