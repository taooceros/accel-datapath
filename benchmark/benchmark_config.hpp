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

// Benchmark configuration from command-line options or TOML file
struct BenchmarkConfig {
  // Polling mode dimension
  bool run_inline = true;
  bool run_threaded = true;

  // Scheduling pattern dimension
  bool run_sliding_window = true;           // Semaphore-like: spawn new op as one completes
  bool run_sliding_window_noalloc = true;   // Same but zero-allocation (nest + pre-allocated slots)
  bool run_sliding_window_arena = false;    // Free-list arena (ibverbs/UCX style O(1) slot recycling)
  bool run_batch = false;                   // Spawn N ops, wait all, repeat
  bool run_scoped_workers = false;          // N workers processing sequentially (N allocations)

  // Queue type dimension
  bool run_nolock = true;
  bool run_mutex = true;
  bool run_tas = true;
  bool run_ttas = true;
  bool run_backoff = true;
  bool run_lockfree = true;

  // Operation dimension
  std::vector<OperationType> operations = all_operations();

  const std::vector<OperationType>& enabled_operations() const;

  // Benchmark parameters
  std::vector<size_t> concurrency_levels = {1, 4, 16, 32};  // Max operations in-flight
  std::vector<size_t> msg_sizes = {256, 512, 1024, 2048, 4096, 8192, 16384};
  size_t total_bytes = 32ULL * 1024 * 1024;  // Total bytes to copy per iteration
  size_t max_ops = 0;  // Max operations per iteration (0 = unlimited, use total_bytes)
  int iterations = 10;  // Number of times to repeat the full copy

  // Output configuration
  std::string csv_file = "dsa_benchmark_results.csv";
};

// Parse command-line arguments and optional TOML config file
BenchmarkConfig parse_args(int argc, char **argv);

// Print usage information
void print_usage(const char *prog);

#endif // BENCHMARK_CONFIG_HPP
