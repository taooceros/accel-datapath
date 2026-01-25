#pragma once
#ifndef BENCHMARK_HELPERS_HPP
#define BENCHMARK_HELPERS_HPP

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <fmt/format.h>
#include <numeric>
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

#endif // BENCHMARK_HELPERS_HPP
