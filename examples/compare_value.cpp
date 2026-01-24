// Example: DSA Compare Value
// Compares a memory region against a 64-bit pattern using Intel DSA

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/compare_value.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <vector>
#include <cstdint>

int main() {
  Dsa dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Test case 1: Buffer filled with matching pattern
  uint64_t pattern = 0xAAAAAAAAAAAAAAAA;
  std::vector<uint64_t> buffer1(4, pattern);

  fmt::println("Test 1: Buffer filled with matching pattern");
  fmt::println("  Pattern: 0x{:016X}", pattern);
  fmt::println("  Buffer contents:");
  for (size_t i = 0; i < buffer1.size(); ++i) {
    fmt::println("    [{}]: 0x{:016X}", i, buffer1[i]);
  }

  auto sender1 =
      dsa_stdexec::dsa_compare_value(dsa, buffer1.data(),
                                      buffer1.size() * sizeof(uint64_t), pattern) |
      stdexec::then([](bool all_match) {
        fmt::println("  Result: {}", all_match ? "ALL MATCH" : "MISMATCH FOUND");
      });

  dsa_stdexec::wait_start(std::move(sender1), loop);

  // Test case 2: Buffer with one different value
  std::vector<uint64_t> buffer2(4, pattern);
  buffer2[2] = 0xBBBBBBBBBBBBBBBB;  // Different value

  fmt::println("\nTest 2: Buffer with one different value");
  fmt::println("  Pattern: 0x{:016X}", pattern);
  fmt::println("  Buffer contents:");
  for (size_t i = 0; i < buffer2.size(); ++i) {
    fmt::println("    [{}]: 0x{:016X}", i, buffer2[i]);
  }

  auto sender2 =
      dsa_stdexec::dsa_compare_value(dsa, buffer2.data(),
                                      buffer2.size() * sizeof(uint64_t), pattern) |
      stdexec::then([](bool all_match) {
        fmt::println("  Result: {}", all_match ? "ALL MATCH" : "MISMATCH FOUND");
      });

  dsa_stdexec::wait_start(std::move(sender2), loop);

  return 0;
}
