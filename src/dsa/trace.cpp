#include "dsa_stdexec/trace.hpp"
#include <perfetto.h>

PERFETTO_TRACK_EVENT_STATIC_STORAGE();

void init_tracing() {
  perfetto::TracingInitArgs args;
  args.backends = perfetto::kSystemBackend;
  perfetto::Tracing::Initialize(args);
  perfetto::TrackEvent::Register();
}

void stop_tracing(const char* /*output_path*/) {
  // With system backend, trace is collected by traced daemon
  // Use perfetto CLI to stop and save the trace
  perfetto::TrackEvent::Flush();
}
