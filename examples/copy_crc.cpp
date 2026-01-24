// Example: DSA Copy with CRC
// Copies data and generates CRC-32C simultaneously using Intel DSA

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/copy_crc.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <string>
#include <cstdint>

int main() {
  Dsa dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Source and destination buffers
  std::string src = "Hello, Copy+CRC World!";
  std::string dst(src.size(), '\0');

  fmt::println("Source: \"{}\"", src);
  fmt::println("Source length: {} bytes", src.size());
  fmt::println("\nCopying data and generating CRC-32C simultaneously...");

  auto sender =
      dsa_stdexec::dsa_copy_crc(dsa, src.data(), dst.data(), src.size()) |
      stdexec::then([&dst](uint32_t crc) {
        fmt::println("\nCopy+CRC complete!");
        fmt::println("Destination: \"{}\"", dst);
        fmt::println("CRC-32C: 0x{:08X}", crc);
      });

  dsa_stdexec::wait_start(std::move(sender), loop);

  // Verify the copy
  fmt::println("\nVerification: src == dst? {}", src == dst ? "YES" : "NO");

  return 0;
}
