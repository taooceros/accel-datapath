#pragma once
#ifndef TRACE_HPP
#define TRACE_HPP

#ifdef DSA_ENABLE_TRACING
#include <perfetto.h>

PERFETTO_DEFINE_CATEGORIES(
    PERFETTO_CATEGORY(dsa),
    PERFETTO_CATEGORY(app_finished)
);

void init_tracing();
void stop_tracing(const char* output_path);

#else
// No-op stubs when tracing is disabled
#define TRACE_EVENT(...)
#define TRACE_EVENT_BEGIN(...)
#define TRACE_EVENT_END(...)
inline void init_tracing() {}
inline void stop_tracing(const char*) {}
#endif

#endif
