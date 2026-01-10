#pragma once
#ifndef TRACE_HPP
#define TRACE_HPP
#include <perfetto.h>

PERFETTO_DEFINE_CATEGORIES(
    PERFETTO_CATEGORY(dsa),
    PERFETTO_CATEGORY(app_finished)
);

void init_tracing();
void stop_tracing(const char* output_path);
#endif
