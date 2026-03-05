// Example: DSA Dualcast
// Copies data to two destinations simultaneously using Intel DSA
// Note: Both destination addresses must have the same bits 11:0 (4KB page offset)

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/dualcast.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <cstdlib>
#include <cstring>
#include <string>

int main() {
  Dsa dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Source data
  std::string src = "Hello, Dualcast!";

  // Allocate aligned destinations (same page offset required)
  // We allocate larger buffers and find addresses with matching bits 11:0
  constexpr size_t alignment = 4096;  // Page size
  constexpr size_t buffer_size = 256;

  void* raw1 = std::aligned_alloc(alignment, buffer_size);
  void* raw2 = std::aligned_alloc(alignment, buffer_size);

  // Both are page-aligned, so bits 11:0 are both 0
  char* dst1 = static_cast<char*>(raw1);
  char* dst2 = static_cast<char*>(raw2);

  std::memset(dst1, 0, buffer_size);
  std::memset(dst2, 0, buffer_size);

  fmt::println("Source: \"{}\"", src);
  fmt::println("Copying to two destinations using dualcast...");
  fmt::println("  dst1 address: {:p} (bits 11:0 = 0x{:03X})",
               static_cast<void*>(dst1),
               reinterpret_cast<uintptr_t>(dst1) & 0xFFF);
  fmt::println("  dst2 address: {:p} (bits 11:0 = 0x{:03X})",
               static_cast<void*>(dst2),
               reinterpret_cast<uintptr_t>(dst2) & 0xFFF);

  auto sender =
      dsa_stdexec::dsa_dualcast(dsa, src.data(), dst1, dst2, src.size()) |
      stdexec::then([dst1, dst2] {
        fmt::println("\nDualcast complete!");
        fmt::println("  Destination 1: \"{}\"", dst1);
        fmt::println("  Destination 2: \"{}\"", dst2);
      });

  dsa_stdexec::wait_start(std::move(sender), loop);

  std::free(raw1);
  std::free(raw2);

  return 0;
}
