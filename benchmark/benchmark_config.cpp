#include "benchmark_config.hpp"
// Include fmt/format.h BEFORE toml++ to ensure std::string formatter
// specialization is defined before toml++ can instantiate it
#include <fmt/format.h>
#include <cstdlib>
#include <toml++/toml.hpp>

const char* operation_name(OperationType op) {
  switch (op) {
    case OperationType::DataMove:     return "data_move";
    case OperationType::MemFill:      return "mem_fill";
    case OperationType::Compare:      return "compare";
    case OperationType::CompareValue: return "compare_value";
    case OperationType::Dualcast:     return "dualcast";
    case OperationType::CrcGen:       return "crc_gen";
    case OperationType::CopyCrc:      return "copy_crc";
    case OperationType::CacheFlush:   return "cache_flush";
  }
  return "unknown";
}

std::vector<OperationType> all_operations() {
  return {
    OperationType::DataMove, OperationType::MemFill, OperationType::Compare,
    OperationType::CompareValue, OperationType::Dualcast, OperationType::CrcGen,
    OperationType::CopyCrc, OperationType::CacheFlush
  };
}

std::optional<OperationType> parse_operation_name(std::string_view name) {
  for (auto op : all_operations()) {
    if (name == operation_name(op)) return op;
  }
  return std::nullopt;
}

const std::vector<OperationType>& BenchmarkConfig::enabled_operations() const {
  return operations;
}

// Load configuration from TOML file
static BenchmarkConfig load_config_from_toml(const std::string &filename) {
  BenchmarkConfig config;

  toml::table tbl;
  try {
    tbl = toml::parse_file(filename);
  } catch (const toml::parse_error &err) {
    fmt::println(stderr, "Failed to parse config file '{}': {}", filename, err.what());
    std::exit(1);
  }

  // Helper: safely get a bool from a TOML table, returning default if key missing
  auto get_bool = [](const toml::table* t, std::string_view key, bool def) {
    if (auto node = t->get(key)) return node->value_or(def);
    return def;
  };

  // Polling mode
  if (auto polling = tbl["polling"].as_table()) {
    config.run_inline = get_bool(polling, "inline", true);
    config.run_threaded = get_bool(polling, "threaded", true);
  }

  // Scheduling pattern
  if (auto scheduling = tbl["scheduling"].as_table()) {
    config.run_sliding_window = get_bool(scheduling, "sliding_window", true);
    config.run_sliding_window_noalloc = get_bool(scheduling, "sliding_window_noalloc", true);
    config.run_batch = get_bool(scheduling, "batch", false);
    config.run_scoped_workers = get_bool(scheduling, "scoped_workers", false);
  }

  // Queue types
  if (auto queues = tbl["queues"].as_table()) {
    config.run_nolock = get_bool(queues, "nolock", true);
    config.run_mutex = get_bool(queues, "mutex", true);
    config.run_tas = get_bool(queues, "tas", true);
    config.run_ttas = get_bool(queues, "ttas", true);
    config.run_backoff = get_bool(queues, "backoff", true);
    config.run_lockfree = get_bool(queues, "lockfree", true);
  }

  // Operations
  if (auto operations = tbl["operations"].as_table()) {
    auto enabled_node = operations->get("enabled");
    if (enabled_node && enabled_node->as_array()) {
      auto arr = enabled_node->as_array();
      config.operations.clear();
      for (const auto &elem : *arr) {
        if (auto name = elem.value<std::string>()) {
          if (auto op = parse_operation_name(*name)) {
            config.operations.push_back(*op);
          } else {
            fmt::println(stderr, "Unknown operation in config: {}", *name);
            std::exit(1);
          }
        }
      }
    }
  }

  // Helper: safely get an int64 from a TOML table
  auto get_int = [](const toml::table* t, std::string_view key) -> std::optional<int64_t> {
    if (auto node = t->get(key)) return node->value<int64_t>();
    return std::nullopt;
  };
  // Helper: safely get an array from a TOML table
  auto get_array = [](const toml::table* t, std::string_view key) -> const toml::array* {
    if (auto node = t->get(key)) return node->as_array();
    return nullptr;
  };

  // Benchmark parameters
  if (auto params = tbl["parameters"].as_table()) {
    if (auto arr = get_array(params, "concurrency_levels")) {
      config.concurrency_levels.clear();
      for (const auto &elem : *arr) {
        if (auto val = elem.value<int64_t>()) {
          config.concurrency_levels.push_back(static_cast<size_t>(*val));
        }
      }
    }
    if (auto arr = get_array(params, "msg_sizes")) {
      config.msg_sizes.clear();
      for (const auto &elem : *arr) {
        if (auto val = elem.value<int64_t>()) {
          config.msg_sizes.push_back(static_cast<size_t>(*val));
        }
      }
    }
    if (auto val = get_int(params, "iterations")) {
      config.iterations = static_cast<int>(*val);
    }
    if (auto val = get_int(params, "total_bytes")) {
      config.total_bytes = static_cast<size_t>(*val);
    }
    if (auto val = get_int(params, "max_ops")) {
      config.max_ops = static_cast<size_t>(*val);
    }
  }

  // Output configuration
  if (auto output = tbl["output"].as_table()) {
    if (auto node = output->get("csv_file")) {
      if (auto val = node->value<std::string>()) {
        config.csv_file = *val;
      }
    }
  }

  return config;
}

void print_usage(const char *prog) {
  fmt::println("Usage: {} [OPTIONS]", prog);
  fmt::println("");
  fmt::println("Options:");
  fmt::println("  --help, -h          Show this help message");
  fmt::println("  --config=<file>     Load configuration from TOML file");
  fmt::println("                      (command-line options override config file)");
  fmt::println("");
  fmt::println("Polling mode (can combine multiple):");
  fmt::println("  --inline            Run inline polling benchmarks (PollingRunLoop)");
  fmt::println("  --threaded          Run background thread polling benchmarks");
  fmt::println("");
  fmt::println("Scheduling pattern (can combine multiple):");
  fmt::println("  --sliding-window    Semaphore-like: spawn new op as one completes (default)");
  fmt::println("  --sliding-window-noalloc  Same as sliding-window but zero-allocation (default)");
  fmt::println("  --batch             Spawn N ops, wait all complete, repeat");
  fmt::println("  --scoped-workers    N worker coroutines processing sequentially");
  fmt::println("");
  fmt::println("Queue types:");
  fmt::println("  --queue=<type>      Run only specified queue type(s), comma-separated");
  fmt::println("                      Types: nolock, mutex, tas, ttas, backoff, lockfree");
  fmt::println("");
  fmt::println("Operations:");
  fmt::println("  --operation=<type>  Run only specified operation(s), comma-separated");
  fmt::println("                      Types: data_move, mem_fill, compare, compare_value,");
  fmt::println("                             dualcast, crc_gen, copy_crc, cache_flush");
  fmt::println("");
  fmt::println("Examples:");
  fmt::println("  {}                                  # Default: sliding-window with inline+threaded", prog);
  fmt::println("  {} --config=benchmark_config.toml   # Load from TOML config file", prog);
  fmt::println("  {} --inline                         # Only inline polling", prog);
  fmt::println("  {} --threaded                       # Only background thread polling", prog);
  fmt::println("  {} --batch                          # Only batch pattern", prog);
  fmt::println("  {} --scoped-workers --inline        # Scoped workers with inline only", prog);
  fmt::println("  {} --sliding-window --batch         # Both sliding-window and batch", prog);
  fmt::println("  {} --queue=mutex,lockfree           # Specific queue types", prog);
}

BenchmarkConfig parse_args(int argc, char **argv) {
  BenchmarkConfig config;
  bool polling_specified = false;
  bool pattern_specified = false;
  bool queue_specified = false;
  bool operation_specified = false;
  std::string config_file;

  // First pass: check for config file
  for (int i = 1; i < argc; ++i) {
    std::string arg = argv[i];
    if (arg.starts_with("--config=")) {
      config_file = arg.substr(9);
      break;
    }
  }

  // Load config from file if specified
  if (!config_file.empty()) {
    config = load_config_from_toml(config_file);
  }

  // Second pass: override with command-line options
  for (int i = 1; i < argc; ++i) {
    std::string arg = argv[i];

    if (arg == "--help" || arg == "-h") {
      print_usage(argv[0]);
      std::exit(0);
    } else if (arg.starts_with("--config=")) {
      // Already handled in first pass
      continue;
    } else if (arg == "--inline") {
      if (!polling_specified) {
        config.run_inline = false;
        config.run_threaded = false;
        polling_specified = true;
      }
      config.run_inline = true;
    } else if (arg == "--threaded") {
      if (!polling_specified) {
        config.run_inline = false;
        config.run_threaded = false;
        polling_specified = true;
      }
      config.run_threaded = true;
    } else if (arg == "--sliding-window") {
      if (!pattern_specified) {
        config.run_sliding_window = false;
        config.run_sliding_window_noalloc = false;
        config.run_batch = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_sliding_window = true;
    } else if (arg == "--sliding-window-noalloc") {
      if (!pattern_specified) {
        config.run_sliding_window = false;
        config.run_sliding_window_noalloc = false;
        config.run_batch = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_sliding_window_noalloc = true;
    } else if (arg == "--batch") {
      if (!pattern_specified) {
        config.run_sliding_window = false;
        config.run_sliding_window_noalloc = false;
        config.run_batch = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_batch = true;
    } else if (arg == "--scoped-workers") {
      if (!pattern_specified) {
        config.run_sliding_window = false;
        config.run_sliding_window_noalloc = false;
        config.run_batch = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_scoped_workers = true;
    } else if (arg.starts_with("--queue=")) {
      if (!queue_specified) {
        config.run_nolock = false;
        config.run_mutex = false;
        config.run_tas = false;
        config.run_ttas = false;
        config.run_backoff = false;
        config.run_lockfree = false;
        queue_specified = true;
      }
      std::string queues = arg.substr(8);
      size_t pos = 0;
      while (pos < queues.size()) {
        size_t end = queues.find(',', pos);
        if (end == std::string::npos)
          end = queues.size();
        std::string q = queues.substr(pos, end - pos);
        if (q == "nolock")
          config.run_nolock = true;
        else if (q == "mutex")
          config.run_mutex = true;
        else if (q == "tas")
          config.run_tas = true;
        else if (q == "ttas")
          config.run_ttas = true;
        else if (q == "backoff")
          config.run_backoff = true;
        else if (q == "lockfree")
          config.run_lockfree = true;
        else {
          fmt::println(stderr, "Unknown queue type: {}", q);
          std::exit(1);
        }
        pos = end + 1;
      }
    } else if (arg.starts_with("--operation=")) {
      if (!operation_specified) {
        config.operations.clear();
        operation_specified = true;
      }
      std::string ops = arg.substr(12);
      size_t pos = 0;
      while (pos < ops.size()) {
        size_t end = ops.find(',', pos);
        if (end == std::string::npos)
          end = ops.size();
        std::string o = ops.substr(pos, end - pos);
        if (auto op = parse_operation_name(o)) {
          config.operations.push_back(*op);
        } else {
          fmt::println(stderr, "Unknown operation type: {}", o);
          std::exit(1);
        }
        pos = end + 1;
      }
    } else {
      fmt::println(stderr, "Unknown option: {}", arg);
      print_usage(argv[0]);
      std::exit(1);
    }
  }

  return config;
}
