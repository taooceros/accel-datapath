// Example: DSA Cache Flush
// Flushes CPU cache lines for a memory region using Intel DSA

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/cache_flush.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <vector>
#include <cstdint>

int main() {
  Dsa dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Create a buffer to flush
  constexpr size_t buffer_size = 4096;  // One page
  std::vector<uint8_t> buffer(buffer_size);

  // Fill with some data
  for (size_t i = 0; i < buffer.size(); ++i) {
    buffer[i] = static_cast<uint8_t>(i & 0xFF);
  }

  fmt::println("Buffer size: {} bytes", buffer.size());
  fmt::println("Flushing cache lines using DSA...");

  auto sender =
      dsa_stdexec::dsa_cache_flush(dsa, buffer.data(), buffer.size()) |
      stdexec::then([] {
        fmt::println("Cache flush complete!");
        fmt::println("All cache lines for the buffer have been written back to memory.");
      });

  dsa_stdexec::wait_start(std::move(sender), loop);

  return 0;
}
