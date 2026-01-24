// Example: DSA CRC Generation
// Generates CRC-32C checksum over a memory region using Intel DSA

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/crc_gen.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <string>
#include <cstdint>

int main() {
  Dsa dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Test data
  std::string data = "Hello, CRC World!";

  fmt::println("Data: \"{}\"", data);
  fmt::println("Data length: {} bytes", data.size());
  fmt::println("\nGenerating CRC-32C using DSA...");

  auto sender =
      dsa_stdexec::dsa_crc_gen(dsa, data.data(), data.size()) |
      stdexec::then([](uint32_t crc) {
        fmt::println("CRC-32C: 0x{:08X}", crc);
      });

  dsa_stdexec::wait_start(std::move(sender), loop);

  // Test with different data
  std::string data2 = "The quick brown fox jumps over the lazy dog";

  fmt::println("\n---");
  fmt::println("Data: \"{}\"", data2);
  fmt::println("Data length: {} bytes", data2.size());
  fmt::println("\nGenerating CRC-32C using DSA...");

  auto sender2 =
      dsa_stdexec::dsa_crc_gen(dsa, data2.data(), data2.size()) |
      stdexec::then([](uint32_t crc) {
        fmt::println("CRC-32C: 0x{:08X}", crc);
      });

  dsa_stdexec::wait_start(std::move(sender2), loop);

  // Test with custom seed
  uint32_t seed = 0xFFFFFFFF;
  fmt::println("\n---");
  fmt::println("Testing with custom seed: 0x{:08X}", seed);

  auto sender3 =
      dsa_stdexec::dsa_crc_gen(dsa, data.data(), data.size(), seed) |
      stdexec::then([](uint32_t crc) {
        fmt::println("CRC-32C (with seed): 0x{:08X}", crc);
      });

  dsa_stdexec::wait_start(std::move(sender3), loop);

  return 0;
}
