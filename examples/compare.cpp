// Example: DSA Compare
// Compares two memory regions using Intel DSA

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/compare.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <string>

int main() {
  Dsa dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Test case 1: Equal buffers
  std::string buf1 = "Hello, World!";
  std::string buf2 = "Hello, World!";

  fmt::println("Test 1: Comparing equal buffers");
  fmt::println("  Buffer 1: \"{}\"", buf1);
  fmt::println("  Buffer 2: \"{}\"", buf2);

  auto sender1 =
      dsa_stdexec::dsa_compare(dsa, buf1.data(), buf2.data(), buf1.size()) |
      stdexec::then([](bool equal) {
        fmt::println("  Result: {}", equal ? "EQUAL" : "NOT EQUAL");
      });

  dsa_stdexec::wait_start(std::move(sender1), loop);

  // Test case 2: Different buffers
  std::string buf3 = "Hello, World!";
  std::string buf4 = "Hello, DSA!!";

  fmt::println("\nTest 2: Comparing different buffers");
  fmt::println("  Buffer 1: \"{}\"", buf3);
  fmt::println("  Buffer 2: \"{}\"", buf4);

  auto sender2 =
      dsa_stdexec::dsa_compare(dsa, buf3.data(), buf4.data(), buf3.size()) |
      stdexec::then([](bool equal) {
        fmt::println("  Result: {}", equal ? "EQUAL" : "NOT EQUAL");
      });

  dsa_stdexec::wait_start(std::move(sender2), loop);

  return 0;
}
