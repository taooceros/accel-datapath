#include <chrono>
#include <cstdlib>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa_stdexec/data_move.hpp>
#include <exec/async_scope.hpp>
#include <fmt/base.h>
#include <fmt/core.h>
#include <fmt/ranges.h>
#include <latch>
#include <stdexec/execution.hpp>
#include <utility>
#include <vector>
#include <thread>

// Helper to recursively build senders to avoid hitting tuple size limits in
// stdexec
template <size_t Offset, size_t Remaining>
auto make_batch(Dsa &dsa, size_t msg_size, char *src_base, char *dst_base) {
  if constexpr (Remaining <= 8) {
    return [&]<size_t... Is>(std::index_sequence<Is...>) {
      return stdexec::when_all(dsa_stdexec::dsa_data_move(
          dsa, src_base + (Offset + Is) * msg_size,
          dst_base + (Offset + Is) * msg_size, msg_size)...);
    }(std::make_index_sequence<Remaining>{});
  } else {
    constexpr size_t Half = Remaining / 2;
    return stdexec::when_all(
        make_batch<Offset, Half>(dsa, msg_size, src_base, dst_base),
        make_batch<Offset + Half, Remaining - Half>(dsa, msg_size, src_base,
                                                    dst_base));
  }
}

// Helper to run a batch of operations using static composition
template <size_t BatchSize>
void run_static_batch_impl(Dsa &dsa, size_t msg_size, char *src, char *dst) {
  auto senders = make_batch<0, BatchSize>(dsa, msg_size, src, dst);
  stdexec::sync_wait(std::move(senders));
}

void run_static_batch(Dsa &dsa, size_t batch_size, size_t msg_size,
                      std::vector<char> &src_vec, std::vector<char> &dst_vec) {
  char *src = src_vec.data();
  char *dst = dst_vec.data();
  if (batch_size == 1)
    run_static_batch_impl<1>(dsa, msg_size, src, dst);
  else if (batch_size == 2)
    run_static_batch_impl<2>(dsa, msg_size, src, dst);
  else if (batch_size == 4)
    run_static_batch_impl<4>(dsa, msg_size, src, dst);
  else if (batch_size == 8)
    run_static_batch_impl<8>(dsa, msg_size, src, dst);
  else if (batch_size == 16)
    run_static_batch_impl<16>(dsa, msg_size, src, dst);
  else if (batch_size == 32)
    run_static_batch_impl<32>(dsa, msg_size, src, dst);
  else {
    // fmt::println("Static batching unsupported size: {}", batch_size);
  }
}

// Helper to run a batch of operations using dynamic spawning
auto run_dynamic_batch(Dsa &dsa, size_t batch_size, size_t msg_size,
                       std::vector<char> &src, std::vector<char> &dst) {
  exec::async_scope scope;
  for (size_t i = 0; i < batch_size; ++i) {
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + i * msg_size,
                                          dst.data() + i * msg_size, msg_size);

    scope.spawn(snd);
  }

  stdexec::sync_wait(scope.on_empty());
}

void benchmark(Dsa &dsa, size_t batch_size, size_t msg_size) {
  // Allocate memory
  size_t total_size = batch_size * msg_size;

  std::vector<char> src(total_size);
  std::vector<char> dst(total_size);

  // Fill src
  std::memset(src.data(), 1, total_size);
  std::memset(dst.data(), 0, total_size);

  // --- Memcpy Benchmark ---
  // Reset dst
  std::memset(dst.data(), 0, total_size);

  // Warmup
  std::memcpy(dst.data(), src.data(), total_size);

  constexpr int iterations = 100;

  auto start_memcpy = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    std::memcpy(dst.data(), src.data(), total_size);
  }
  auto end_memcpy = std::chrono::high_resolution_clock::now();

  std::chrono::duration<double> diff_memcpy = end_memcpy - start_memcpy;
  double bw_memcpy = (double)total_size * iterations /
                     (1024.0 * 1024.0 * 1024.0) / diff_memcpy.count();

  // --- Static Benchmark ---
  double bw_static = 0.0;
  if (batch_size <= 32) {
    // Warmup
    run_static_batch(dsa, batch_size, msg_size, src, dst);

    auto start_static = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < iterations; ++i) {
      run_static_batch(dsa, batch_size, msg_size, src, dst);
    }
    auto end_static = std::chrono::high_resolution_clock::now();

    std::chrono::duration<double> diff_static = end_static - start_static;
    bw_static = (double)total_size * iterations / (1024.0 * 1024.0 * 1024.0) /
                diff_static.count();
  }

  // --- Dynamic Benchmark ---
  // Reset dst (optional, but good practice)
  std::memset(dst.data(), 0, total_size);

  // Warmup
  run_dynamic_batch(dsa, batch_size, msg_size, src, dst);

  auto start_dynamic = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    run_dynamic_batch(dsa, batch_size, msg_size, src, dst);
  }
  auto end_dynamic = std::chrono::high_resolution_clock::now();

  std::chrono::duration<double> diff_dynamic = end_dynamic - start_dynamic;
  double bw_dynamic = (double)total_size * iterations /
                      (1024.0 * 1024.0 * 1024.0) / diff_dynamic.count();
  if (batch_size <= 32) {
    fmt::println("Batch: {:3}, Size: {:8} bytes | Memcpy: {:.2f} GB/s | Static: "
                 "{:.2f} GB/s | Dynamic: {:.2f} GB/s",
                 batch_size, msg_size, bw_memcpy, bw_static, bw_dynamic);
  } else {
    fmt::println("Batch: {:3}, Size: {:8} bytes | Memcpy: {:.2f} GB/s | Static: "
                 "N/A        | Dynamic: {:.2f} GB/s",
                 batch_size, msg_size, bw_memcpy, bw_dynamic);
  }
}

int main(int argc, char **argv) {
  std::system("stty opost onlcr");
  try {
    // Check arguments if we want to customize, but defaults are fine.
    bool use_poller = true;

    fmt::println("Initializing DSA (poller={})...", use_poller);
    Dsa dsa(use_poller);

    std::vector<size_t> batch_sizes = {1, 2, 4, 8, 16, 32};
    std::vector<size_t> msg_sizes = {1024, 4096, 64 * 1024, 1024 * 1024};

    fmt::println("Starting DSA Benchmark...");

    for (auto bs : batch_sizes) {
      for (auto ms : msg_sizes) {
        // Ensure we don't allocate absurd amounts of memory (e.g. 256 * 16MB =
        // 4GB, which is fine but let's be aware)
        if (bs * ms > 2ULL * 1024 * 1024 * 1024) {
          fmt::println("Skipping Batch: {}, Size: {} (Total > 2GB)", bs, ms);
          continue;
        }
        // std::this_thread::sleep_for(std::chrono::milliseconds(100));
        benchmark(dsa, bs, ms);
      }
    }
  } catch (const std::exception &e) {
    fmt::println(stderr, "Error: {}", e.what());
    return 1;
  }

  fmt::println("Benchmark completed.");
  return 0;
}
