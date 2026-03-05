#include "dsa/dsa.hpp"
#include "fmt/base.h"

#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/scheduler.hpp>
#include <exec/async_scope.hpp>
#include <stdexec/execution.hpp>

#include <dsa_stdexec/submit.hpp>
#include <dsa_stdexec/sync_wait.hpp>

int main() {
  Dsa dsa(false); // Disable background poller
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  std::string src = "Hello, World!";
  std::string dst(src.size(), 0);

  auto snd =
      dsa_stdexec::dsa_data_move(dsa, src.data(), dst.data(), src.size()) |
      stdexec::then([&dst] { fmt::println("job finished: output {}", dst); });

  dsa_stdexec::wait_start(std::move(snd), loop);

  return 0;
}
