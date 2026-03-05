// Example: DSA Memory Fill
// Fills a memory region with a 64-bit pattern using Intel DSA

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/mem_fill.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <vector>
#include <cstdint>

int main() {
  Dsa dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Create a buffer to fill
  std::vector<uint64_t> buffer(8, 0);

  // Pattern to fill with (will be repeated as 64-bit values)
  uint64_t pattern = 0xDEADBEEFCAFEBABE;

  fmt::println("Buffer before fill:");
  for (size_t i = 0; i < buffer.size(); ++i) {
    fmt::println("  [{}]: 0x{:016X}", i, buffer[i]);
  }

  fmt::println("\nFilling with pattern 0x{:016X}...", pattern);

  auto sender =
      dsa_stdexec::dsa_mem_fill(dsa, buffer.data(), buffer.size() * sizeof(uint64_t), pattern) |
      stdexec::then([&buffer] {
        fmt::println("\nBuffer after fill:");
        for (size_t i = 0; i < buffer.size(); ++i) {
          fmt::println("  [{}]: 0x{:016X}", i, buffer[i]);
        }
      });

  dsa_stdexec::wait_start(std::move(sender), loop);

  return 0;
}
