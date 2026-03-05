// Example: DSA Data Move (Memory Copy)
// Copies data from source buffer to destination buffer using Intel DSA

#include <dsa/dsa.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <fmt/base.h>
#include <stdexec/execution.hpp>
#include <string>

int main() {
  // Create DSA instance (false = no background polling thread)
  Dsa dsa(false);

  // Create a run loop that polls DSA for completions
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Source and destination buffers
  std::string src = "Hello, DSA World!";
  std::string dst(src.size(), '\0');

  fmt::println("Source: \"{}\"", src);
  fmt::println("Copying {} bytes using DSA...", src.size());

  // Create and execute the data move operation
  auto sender =
      dsa_stdexec::dsa_data_move(dsa, src.data(), dst.data(), src.size()) |
      stdexec::then([&dst] {
        fmt::println("Copy complete!");
        fmt::println("Destination: \"{}\"", dst);
      });

  dsa_stdexec::wait_start(std::move(sender), loop);

  return 0;
}
