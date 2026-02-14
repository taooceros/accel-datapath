#pragma once
#ifndef BENCHMARK_CONFIG_HPP
#define BENCHMARK_CONFIG_HPP

#include <cstddef>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

enum class OperationType {
  DataMove, MemFill, Compare, CompareValue, Dualcast, CrcGen, CopyCrc, CacheFlush
};

const char* operation_name(OperationType op);
std::optional<OperationType> parse_operation_name(std::string_view name);
std::vector<OperationType> all_operations();

enum class PollingMode { Inline, Threaded };

const char* polling_mode_name(PollingMode m);
std::optional<PollingMode> parse_polling_mode(std::string_view name);
std::vector<PollingMode> all_polling_modes();

enum class SchedulingPattern {
  SlidingWindow, SlidingWindowNoAlloc, SlidingWindowArena, Batch, BatchNoAlloc, ScopedWorkers, BatchRaw
};

const char* scheduling_pattern_name(SchedulingPattern p);
std::optional<SchedulingPattern> parse_scheduling_pattern(std::string_view name);
std::vector<SchedulingPattern> default_scheduling_patterns();
std::vector<SchedulingPattern> all_scheduling_patterns();

enum class SubmissionStrategy { Immediate, DoubleBufBatch, FixedRingBatch, RingBatch };

const char* submission_strategy_name(SubmissionStrategy s);
std::optional<SubmissionStrategy> parse_submission_strategy(std::string_view name);
std::vector<SubmissionStrategy> default_submission_strategies();
std::vector<SubmissionStrategy> all_submission_strategies();

enum class QueueType { NoLock, Mutex, TAS, TTAS, Backoff, LockFree };

const char* queue_type_name(QueueType q);
std::optional<QueueType> parse_queue_type(std::string_view name);
std::vector<QueueType> all_queue_types();

// Benchmark configuration from command-line options or TOML file
struct BenchmarkConfig {
  std::vector<PollingMode> polling_modes = all_polling_modes();
  std::vector<SchedulingPattern> scheduling_patterns = default_scheduling_patterns();
  std::vector<SubmissionStrategy> submission_strategies = default_submission_strategies();
  std::vector<QueueType> queue_types = all_queue_types();
  std::vector<OperationType> operations = all_operations();

  // Benchmark parameters
  std::vector<size_t> concurrency_levels = {1, 4, 16, 32};
  std::vector<size_t> msg_sizes = {256, 512, 1024, 2048, 4096, 8192, 16384};
  size_t total_bytes = 32ULL * 1024 * 1024;
  size_t max_ops = 0;
  int iterations = 10;

  // Latency sampling
  bool sample_latency = true;

  // Output configuration
  std::string csv_file = "dsa_benchmark_results.csv";

  // Convenience: check if a specific enum value is in the corresponding vector
  bool has_polling(PollingMode m) const;
  bool has_pattern(SchedulingPattern p) const;
  bool has_submission(SubmissionStrategy s) const;
  bool has_queue(QueueType q) const;
};

// Parse command-line arguments and optional TOML config file
BenchmarkConfig parse_args(int argc, char **argv);

// Print usage information
void print_usage(const char *prog);

#endif // BENCHMARK_CONFIG_HPP
