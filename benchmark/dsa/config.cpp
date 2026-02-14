#include "config.hpp"
// Include fmt/format.h BEFORE toml++ to ensure std::string formatter
// specialization is defined before toml++ can instantiate it
#include <fmt/format.h>
#include <algorithm>
#include <cstdlib>
#include <optional>
#include <toml++/toml.hpp>

// ============================================================================
// OperationType
// ============================================================================

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

// ============================================================================
// PollingMode
// ============================================================================

const char* polling_mode_name(PollingMode m) {
  switch (m) {
    case PollingMode::Inline:   return "inline";
    case PollingMode::Threaded: return "threaded";
  }
  return "unknown";
}

std::optional<PollingMode> parse_polling_mode(std::string_view name) {
  for (auto m : all_polling_modes()) {
    if (name == polling_mode_name(m)) return m;
  }
  return std::nullopt;
}

std::vector<PollingMode> all_polling_modes() {
  return {PollingMode::Inline, PollingMode::Threaded};
}

// ============================================================================
// SchedulingPattern
// ============================================================================

const char* scheduling_pattern_name(SchedulingPattern p) {
  switch (p) {
    case SchedulingPattern::SlidingWindow:        return "sliding_window";
    case SchedulingPattern::SlidingWindowNoAlloc: return "sliding_window_noalloc";
    case SchedulingPattern::SlidingWindowArena:   return "sliding_window_arena";
    case SchedulingPattern::Batch:                return "batch";
    case SchedulingPattern::BatchNoAlloc:         return "batch_noalloc";
    case SchedulingPattern::ScopedWorkers:        return "scoped_workers";
    case SchedulingPattern::BatchRaw:             return "batch_raw";
  }
  return "unknown";
}

std::optional<SchedulingPattern> parse_scheduling_pattern(std::string_view name) {
  for (auto p : all_scheduling_patterns()) {
    if (name == scheduling_pattern_name(p)) return p;
  }
  return std::nullopt;
}

std::vector<SchedulingPattern> default_scheduling_patterns() {
  return {SchedulingPattern::SlidingWindow, SchedulingPattern::SlidingWindowNoAlloc};
}

std::vector<SchedulingPattern> all_scheduling_patterns() {
  return {
    SchedulingPattern::SlidingWindow, SchedulingPattern::SlidingWindowNoAlloc,
    SchedulingPattern::SlidingWindowArena, SchedulingPattern::Batch,
    SchedulingPattern::BatchNoAlloc, SchedulingPattern::ScopedWorkers,
    SchedulingPattern::BatchRaw
  };
}

// ============================================================================
// SubmissionStrategy
// ============================================================================

const char* submission_strategy_name(SubmissionStrategy s) {
  switch (s) {
    case SubmissionStrategy::Immediate:      return "immediate";
    case SubmissionStrategy::DoubleBufBatch: return "double_buf_batch";
    case SubmissionStrategy::FixedRingBatch: return "fixed_ring_batch";
    case SubmissionStrategy::RingBatch:      return "ring_batch";
  }
  return "unknown";
}

std::optional<SubmissionStrategy> parse_submission_strategy(std::string_view name) {
  for (auto s : all_submission_strategies()) {
    if (name == submission_strategy_name(s)) return s;
  }
  return std::nullopt;
}

std::vector<SubmissionStrategy> default_submission_strategies() {
  return {SubmissionStrategy::Immediate};
}

std::vector<SubmissionStrategy> all_submission_strategies() {
  return {SubmissionStrategy::Immediate, SubmissionStrategy::DoubleBufBatch,
          SubmissionStrategy::FixedRingBatch, SubmissionStrategy::RingBatch};
}

// ============================================================================
// QueueType
// ============================================================================

const char* queue_type_name(QueueType q) {
  switch (q) {
    case QueueType::NoLock:   return "nolock";
    case QueueType::Mutex:    return "mutex";
    case QueueType::TAS:      return "tas";
    case QueueType::TTAS:     return "ttas";
    case QueueType::Backoff:  return "backoff";
    case QueueType::LockFree: return "lockfree";
  }
  return "unknown";
}

std::optional<QueueType> parse_queue_type(std::string_view name) {
  for (auto q : all_queue_types()) {
    if (name == queue_type_name(q)) return q;
  }
  return std::nullopt;
}

std::vector<QueueType> all_queue_types() {
  return {
    QueueType::NoLock, QueueType::Mutex, QueueType::TAS,
    QueueType::TTAS, QueueType::Backoff, QueueType::LockFree
  };
}

// ============================================================================
// BenchmarkConfig helpers
// ============================================================================

template <typename T>
static bool vec_contains(const std::vector<T> &v, T val) {
  return std::find(v.begin(), v.end(), val) != v.end();
}

bool BenchmarkConfig::has_polling(PollingMode m) const { return vec_contains(polling_modes, m); }
bool BenchmarkConfig::has_pattern(SchedulingPattern p) const { return vec_contains(scheduling_patterns, p); }
bool BenchmarkConfig::has_submission(SubmissionStrategy s) const { return vec_contains(submission_strategies, s); }
bool BenchmarkConfig::has_queue(QueueType q) const { return vec_contains(queue_types, q); }

// ============================================================================
// TOML parsing helpers
// ============================================================================

// Parse a TOML array of strings into an enum vector using a parse function.
// If the key is missing, returns nullopt (caller keeps defaults).
template <typename T>
static std::optional<std::vector<T>> parse_toml_enum_array(
    const toml::table *tbl, std::string_view key,
    std::optional<T>(*parse_fn)(std::string_view),
    const char *context) {
  auto node = tbl->get(key);
  if (!node) return std::nullopt;
  auto arr = node->as_array();
  if (!arr) return std::nullopt;

  std::vector<T> result;
  for (const auto &elem : *arr) {
    if (auto name = elem.value<std::string>()) {
      if (auto val = parse_fn(*name)) {
        result.push_back(*val);
      } else {
        fmt::println(stderr, "Unknown {} in config: {}", context, *name);
        std::exit(1);
      }
    }
  }
  return result;
}

// Parse a TOML table of bool flags into an enum vector.
// Each key in the table maps to an enum value via parse_fn.
// Only keys with value=true are included.
template <typename T>
static std::vector<T> parse_toml_bool_table(
    const toml::table *tbl,
    std::optional<T>(*parse_fn)(std::string_view),
    const std::vector<T> &defaults) {
  std::vector<T> result;
  for (auto &[key, val] : *tbl) {
    if (val.value_or(false)) {
      if (auto e = parse_fn(key)) {
        result.push_back(*e);
      }
    }
  }
  return result.empty() ? defaults : result;
}

// ============================================================================
// TOML config loader
// ============================================================================

static BenchmarkConfig load_config_from_toml(const std::string &filename) {
  BenchmarkConfig config;

  toml::table tbl;
  try {
    tbl = toml::parse_file(filename);
  } catch (const toml::parse_error &err) {
    fmt::println(stderr, "Failed to parse config file '{}': {}", filename, err.what());
    std::exit(1);
  }

  if (auto t = tbl["polling"].as_table())
    config.polling_modes = parse_toml_bool_table(t, parse_polling_mode, all_polling_modes());

  if (auto t = tbl["scheduling"].as_table())
    config.scheduling_patterns = parse_toml_bool_table(t, parse_scheduling_pattern, default_scheduling_patterns());

  if (auto t = tbl["submission"].as_table())
    config.submission_strategies = parse_toml_bool_table(t, parse_submission_strategy, default_submission_strategies());

  if (auto t = tbl["queues"].as_table())
    config.queue_types = parse_toml_bool_table(t, parse_queue_type, all_queue_types());

  if (auto ops = tbl["operations"].as_table()) {
    if (auto v = parse_toml_enum_array(ops, "enabled", parse_operation_name, "operation"))
      config.operations = std::move(*v);
  }

  // Helper: safely get an int64 from a TOML table
  auto get_int = [](const toml::table* t, std::string_view key) -> std::optional<int64_t> {
    if (auto node = t->get(key)) return node->value<int64_t>();
    return std::nullopt;
  };
  auto get_array = [](const toml::table* t, std::string_view key) -> const toml::array* {
    if (auto node = t->get(key)) return node->as_array();
    return nullptr;
  };

  if (auto params = tbl["parameters"].as_table()) {
    if (auto arr = get_array(params, "concurrency_levels")) {
      config.concurrency_levels.clear();
      for (const auto &elem : *arr) {
        if (auto val = elem.value<int64_t>())
          config.concurrency_levels.push_back(static_cast<size_t>(*val));
      }
    }
    if (auto arr = get_array(params, "msg_sizes")) {
      config.msg_sizes.clear();
      for (const auto &elem : *arr) {
        if (auto val = elem.value<int64_t>())
          config.msg_sizes.push_back(static_cast<size_t>(*val));
      }
    }
    if (auto val = get_int(params, "iterations"))
      config.iterations = static_cast<int>(*val);
    if (auto val = get_int(params, "total_bytes"))
      config.total_bytes = static_cast<size_t>(*val);
    if (auto val = get_int(params, "max_ops"))
      config.max_ops = static_cast<size_t>(*val);
    if (auto node = params->get("sample_latency")) {
      if (auto val = node->value<bool>())
        config.sample_latency = *val;
    }
  }

  if (auto output = tbl["output"].as_table()) {
    if (auto node = output->get("csv_file")) {
      if (auto val = node->value<std::string>())
        config.csv_file = *val;
    }
  }

  return config;
}

// ============================================================================
// Usage
// ============================================================================

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
  fmt::println("  --sliding-window-arena    Free-list arena (ibverbs/UCX style O(1) recycling)");
  fmt::println("  --batch             Spawn N ops, wait all complete, repeat");
  fmt::println("  --batch-noalloc     Batch pattern with zero-allocation slots");
  fmt::println("  --scoped-workers    N worker coroutines processing sequentially");
  fmt::println("  --batch-raw         Hardware batch descriptor via dsa_batch sender (inline only)");
  fmt::println("");
  fmt::println("Submission strategy (can combine multiple):");
  fmt::println("  --immediate         1:1 doorbell per descriptor (default)");
  fmt::println("  --double-buf-batch  Double-buffered transparent batch submission");
  fmt::println("  --fixed-ring-batch  Fixed-size ring batch (ablation study)");
  fmt::println("  --ring-batch        Ring-buffer based hardware batch submission");
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
  fmt::println("Latency:");
  fmt::println("  --no-latency        Disable per-operation latency sampling");
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

// ============================================================================
// CLI parsing
// ============================================================================

static std::vector<std::string> split_csv(std::string_view s) {
  std::vector<std::string> out;
  size_t pos = 0;
  while (pos < s.size()) {
    size_t end = s.find(',', pos);
    if (end == std::string_view::npos) end = s.size();
    out.emplace_back(s.substr(pos, end - pos));
    pos = end + 1;
  }
  return out;
}

// CLI flag-to-enum mapping for dimensions that use --flag syntax
// (as opposed to --key=val,val syntax)
struct FlagMapping {
  const char *flag;                  // e.g. "--inline"
  const char *enum_name;             // e.g. "inline" — fed to the parse function
};

static const FlagMapping polling_flags[] = {
  {"--inline",   "inline"},
  {"--threaded", "threaded"},
};

static const FlagMapping pattern_flags[] = {
  {"--sliding-window",         "sliding_window"},
  {"--sliding-window-noalloc", "sliding_window_noalloc"},
  {"--sliding-window-arena",   "sliding_window_arena"},
  {"--batch",                  "batch"},
  {"--batch-noalloc",          "batch_noalloc"},
  {"--scoped-workers",         "scoped_workers"},
  {"--batch-raw",              "batch_raw"},
};

static const FlagMapping submission_flags[] = {
  {"--immediate",        "immediate"},
  {"--double-buf-batch", "double_buf_batch"},
  {"--fixed-ring-batch", "fixed_ring_batch"},
  {"--ring-batch",       "ring_batch"},
};

// Try to match `arg` against a flag table. If matched, add the enum name to `collector`.
// Returns true if matched.
static bool try_flag(std::string_view arg, const FlagMapping *flags, size_t count,
                     std::optional<std::vector<std::string>> &collector) {
  for (size_t i = 0; i < count; ++i) {
    if (arg == flags[i].flag) {
      if (!collector) collector.emplace();
      collector->emplace_back(flags[i].enum_name);
      return true;
    }
  }
  return false;
}

// Apply CLI overrides: parse collected string names into enum vectors.
template <typename T>
static void apply_override(const std::optional<std::vector<std::string>> &collected,
                           std::vector<T> &target,
                           std::optional<T>(*parse_fn)(std::string_view),
                           const char *type_name) {
  if (!collected) return;
  target.clear();
  for (auto &name : *collected) {
    if (auto val = parse_fn(name)) {
      target.push_back(*val);
    } else {
      fmt::println(stderr, "Unknown {}: {}", type_name, name);
      std::exit(1);
    }
  }
}

BenchmarkConfig parse_args(int argc, char **argv) {
  BenchmarkConfig config;
  std::string config_file;

  // Collect CLI overrides per dimension
  std::optional<std::vector<std::string>> cli_polling;
  std::optional<std::vector<std::string>> cli_pattern;
  std::optional<std::vector<std::string>> cli_submission;
  std::optional<std::vector<std::string>> cli_queue;
  std::optional<std::vector<std::string>> cli_operation;

  // First pass: config file
  for (int i = 1; i < argc; ++i) {
    std::string arg = argv[i];
    if (arg.starts_with("--config=")) {
      config_file = arg.substr(9);
      break;
    }
  }
  if (!config_file.empty())
    config = load_config_from_toml(config_file);

  // Second pass: collect CLI overrides
  for (int i = 1; i < argc; ++i) {
    std::string_view arg = argv[i];

    if (arg == "--help" || arg == "-h") {
      print_usage(argv[0]);
      std::exit(0);
    }
    if (arg.starts_with("--config="))
      continue;

    if (try_flag(arg, polling_flags, std::size(polling_flags), cli_polling)) continue;
    if (try_flag(arg, pattern_flags, std::size(pattern_flags), cli_pattern)) continue;
    if (try_flag(arg, submission_flags, std::size(submission_flags), cli_submission)) continue;

    if (arg == "--no-latency") {
      config.sample_latency = false;
      continue;
    }

    if (arg.starts_with("--queue=")) {
      for (auto &q : split_csv(arg.substr(8))) {
        if (!cli_queue) cli_queue.emplace();
        cli_queue->push_back(q);
      }
    } else if (arg.starts_with("--operation=")) {
      for (auto &o : split_csv(arg.substr(12))) {
        if (!cli_operation) cli_operation.emplace();
        cli_operation->push_back(o);
      }
    } else {
      fmt::println(stderr, "Unknown option: {}", arg);
      print_usage(argv[0]);
      std::exit(1);
    }
  }

  // Apply: if a dimension was specified on CLI, override that dimension
  apply_override(cli_polling,    config.polling_modes,          parse_polling_mode,          "polling mode");
  apply_override(cli_pattern,    config.scheduling_patterns,    parse_scheduling_pattern,    "scheduling pattern");
  apply_override(cli_submission, config.submission_strategies,  parse_submission_strategy,   "submission strategy");
  apply_override(cli_queue,      config.queue_types,            parse_queue_type,            "queue type");
  apply_override(cli_operation,  config.operations,             parse_operation_name,        "operation");

  return config;
}
