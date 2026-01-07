#include "dsa_stdexec/trace.hpp"
#include <perfetto.h>

PERFETTO_TRACK_EVENT_STATIC_STORAGE();

void init_tracing() {
  perfetto::TracingInitArgs args;
  args.backends = perfetto::kSystemBackend;
  perfetto::Tracing::Initialize(args);
  perfetto::TrackEvent::Register();
}
