#pragma once
#ifndef ENUM_FORMAT_HPP
#define ENUM_FORMAT_HPP

#include <fmt/format.h>

// Formatter for enum accfg_wq_mode (accel_mode)

#include <accel-config/libaccel_config.h>
namespace fmt {
template <> struct formatter<accfg_wq_mode> : formatter<std::string_view> {
  auto format(accfg_wq_mode mode, format_context &ctx) const
      -> format_context::iterator {
    std::string_view name = "error";
    switch (mode) {
    case ACCFG_WQ_SHARED:
      name = "shared";
      break;
    case ACCFG_WQ_DEDICATED:
      name = "dedicated";
      break;
    case ACCFG_WQ_MODE_UNKNOWN:
      name = "unknown";
      break;
    default:
      break;
    }
    return formatter<std::string_view>::format(name, ctx);
  }
};

template <> struct formatter<accfg_wq_state> : formatter<std::string_view> {
  auto format(accfg_wq_state state, format_context &ctx) const
      -> format_context::iterator {
    std::string_view name = "error";
    switch (state) {
    case ACCFG_WQ_DISABLED:
      name = "disabled";
      break;
    case ACCFG_WQ_ENABLED:
      name = "enabled";
      break;
    case ACCFG_WQ_QUIESCING:
      name = "quiescing";
      break;
    case ACCFG_WQ_LOCKED:
      name = "locked";
      break;
    case ACCFG_WQ_UNKNOWN:
      name = "unknown";
      break;
    default:
      break;
    }
    return formatter<std::string_view>::format(name, ctx);
  }
};

} // namespace fmt

#endif