#pragma once
#ifndef DSA_IMPL_IPP
#define DSA_IMPL_IPP

#include "dsa.hpp"
#include "enum_format.hpp"
#include <cerrno>
#include <climits>
#include <cstdint>
#include <cstdlib>
#include <dsa_stdexec/error.hpp>
#include <dsa_stdexec/trace.hpp>
#include <fcntl.h>
#include <fmt/format.h>
#include <sys/mman.h>
#include <system_error>
#include <unistd.h>
#include <x86intrin.h>

#define WQ_PORTAL_SIZE 4096

namespace detail {

inline uint8_t op_status(uint8_t status) {
  return status & DSA_COMP_STATUS_MASK;
}

} // namespace detail

template <dsa::TaskQueue Queue>
DsaBase<Queue>::DsaBase(bool start_poller)
    : ctx_(), wq_(nullptr), wq_portal_(nullptr) {
  try {
    auto &ctx = context();
    accfg_device *device = nullptr;
    accfg_wq *wq = nullptr;
    int device_count = 0;
    int total_wq_count = 0;
    accfg_device_foreach(ctx.get(), device) {
      device_count++;
      const char *devname = accfg_device_get_devname(device);
      int dev_id = accfg_device_get_id(device);
      std::string type_str = accfg_device_get_type_str(device);
      std::system("stty opost onlcr");
      fmt::println("Device {0} ({1}): {2}", dev_id, devname, type_str);
      int wq_count = 0;

      if (type_str != "dsa") {
        continue;
      }

      accfg_wq_foreach(device, wq) {
        wq_count++;
        total_wq_count++;
        std::string wq_name = accfg_wq_get_devname(wq);
        std::string wq_type = accfg_wq_get_type_name(wq);
        auto mode = accfg_wq_get_mode(wq);
        auto state = accfg_wq_get_state(wq);
        uint64_t max_xfer = accfg_wq_get_max_transfer_size(wq);
        unsigned int max_batch = accfg_wq_get_max_batch_size(wq);
        auto ats_disable = accfg_wq_get_ats_disable(wq);
        fmt::println("  WQ {0} ({1}) type={2} mode={3} state={4} max_xfer={5} "
                     "max_batch={6} ats_disable={7}",
                     accfg_wq_get_id(wq), wq_name, wq_type, mode, state,
                     max_xfer, max_batch, ats_disable);

        if (accfg_wq_get_type(wq) != ACCFG_WQT_USER) {
          continue;
        }

        if (state != ACCFG_WQ_ENABLED) {
          continue;
        }
        fmt::println("  Mapping WQ portal for {0}", wq_name);

        void *portal = map_wq(wq);
        if (portal == MAP_FAILED) {
          continue;
        }

        wq_ = wq;
        wq_portal_ = portal;
        mode_ = mode;

        fmt::println("  Mapped WQ portal for {0} at {1}", wq_name, portal);
        break;
      }

      if (wq_portal_ != nullptr) {
        break;
      }
    }
    if (device_count == 0) {
      fmt::println("No DSA/IAX devices found. Ensure devices are enabled and "
                   "WQs configured.");
    } else {
      fmt::println("Total devices: {0}, total WQs: {1}", device_count,
                   total_wq_count);
    }

    if (wq_portal_ == nullptr) {
      throw dsa_stdexec::DsaInitError(
          "Failed to locate and map a usable user work queue portal");
    }

    if (start_poller) {
      running_ = true;
      poller_ = std::thread([this] {
        while (running_) {
          poll();
        }
      });
    }

  } catch (const std::exception &ex) {
    fmt::println("Error: {0}", ex.what());
    throw;
  }
}

template <dsa::TaskQueue Queue>
DsaBase<Queue>::~DsaBase() {
  running_ = false;
  if (poller_.joinable()) {
    poller_.join();
  }

  if (wq_portal_ != nullptr) {
    munmap(wq_portal_, kWqPortalSize);
    wq_portal_ = nullptr;
  }
}

template <dsa::TaskQueue Queue>
void DsaBase<Queue>::data_move(void *src, void *dst, size_t size) {
  TRACE_EVENT("dsa", "data_move", "size", size);
  if (size == 0) {
    return;
  }

  if (wq_portal_ == nullptr) {
    throw dsa_stdexec::DsaError("DSA work queue portal is not mapped");
  }

  if (src == nullptr || dst == nullptr) {
    throw dsa_stdexec::DsaError("DSA data_move received a null pointer");
  }

  if (size > UINT32_MAX) {
    throw dsa_stdexec::DsaError(
        "DSA data_move size exceeds the maximum transfer length");
  }

  struct dsa_completion_record comp __attribute__((aligned(32))) = {};
  struct dsa_hw_desc desc __attribute__((aligned(64))) = {};

  desc.opcode = DSA_OPCODE_MEMMOVE;
  desc.flags = IDXD_OP_FLAG_RCR;
  desc.flags |= IDXD_OP_FLAG_CRAV;
  desc.flags |= IDXD_OP_FLAG_CC;
  desc.xfer_size = static_cast<uint32_t>(size);
  desc.src_addr = reinterpret_cast<uint64_t>(src);
  desc.dst_addr = reinterpret_cast<uint64_t>(dst);
  desc.completion_addr = reinterpret_cast<uint64_t>(&comp);

retry:
  __builtin_memset(&comp, 0, sizeof(comp));

  _mm_sfence();

  if (mode_ == ACCFG_WQ_DEDICATED) {
    _movdir64b(wq_portal_, &desc);
  } else {
    constexpr int kEnqueueSpinLimit = 1 << 20;
    int enqueue_attempts = 0;

    while (_enqcmd(wq_portal_, &desc) != 0) {
      if (++enqueue_attempts >= kEnqueueSpinLimit) {
        throw dsa_stdexec::DsaSubmitError(
            "DSA portal busy - enqueue spin limit exceeded", EBUSY);
      }
      _mm_pause();
    }
  }

  while (comp.status == 0) {
    _mm_pause();
  }

  uint8_t status_code = detail::op_status(comp.status);
  if (status_code != DSA_COMP_SUCCESS) {
    if (status_code == DSA_COMP_PAGE_FAULT_NOBOF) {
      int wr = comp.status & DSA_COMP_STATUS_WRITE;
      volatile char *t;
      t = (char *)comp.fault_addr;
      wr ? *t = *t : *t;
      desc.src_addr += comp.bytes_completed;
      desc.dst_addr += comp.bytes_completed;
      desc.xfer_size -= comp.bytes_completed;
      goto retry;
    }
    throw dsa_stdexec::DsaError(status_code, comp, DSA_OPCODE_MEMMOVE,
                                "data_move");
  }
}

template <dsa::TaskQueue Queue>
void *DsaBase<Queue>::map_wq(accfg_wq *wq) {
  char path[PATH_MAX] = {};
  if (accfg_wq_get_user_dev_path(wq, path, sizeof(path)) != 0) {
    fmt::println("    Failed to get user device path for WQ {0}",
                 accfg_wq_get_id(wq));
    return MAP_FAILED;
  }

  int fd = open(path, O_RDWR);
  if (fd < 0) {
    std::error_code ec(errno, std::generic_category());
    fmt::println("    Failed to open {0}: {1}", path, ec.message());
    return MAP_FAILED;
  }

  void *portal = mmap(nullptr, WQ_PORTAL_SIZE, PROT_WRITE,
                      MAP_SHARED | MAP_POPULATE, fd, 0);
  std::error_code mmap_error;
  if (portal == MAP_FAILED) {
    mmap_error = std::error_code(errno, std::generic_category());
  }

  close(fd);

  if (portal == MAP_FAILED) {
    fmt::println("    Failed to mmap portal {0}: {1}", path,
                 mmap_error.message());
  }

  return portal;
}

template <dsa::TaskQueue Queue>
void DsaBase<Queue>::submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc) {
  TRACE_EVENT("dsa", "submit_hw", "op", (uintptr_t)op);
  if (wq_portal_ == nullptr) {
    throw dsa_stdexec::DsaSubmitError("DSA work queue portal is not mapped");
  }

  task_queue_.push(op);

  _mm_sfence();

  if (mode_ == ACCFG_WQ_DEDICATED) {
    _movdir64b(wq_portal_, desc);
  } else {
    constexpr int kEnqueueSpinLimit = 1 << 20;
    int enqueue_attempts = 0;

    while (_enqcmd(wq_portal_, desc) != 0) {
      if (++enqueue_attempts >= kEnqueueSpinLimit) {
        throw dsa_stdexec::DsaSubmitError(
            "DSA portal busy - enqueue spin limit exceeded", EBUSY);
      }
    }
  }
}

template <dsa::TaskQueue Queue>
void DsaBase<Queue>::submit(dsa_stdexec::OperationBase *op) {
  TRACE_EVENT("dsa", "submit", "op", (uintptr_t)op);
  task_queue_.push(op);
}

template <dsa::TaskQueue Queue>
void DsaBase<Queue>::poll() {
  task_queue_.poll();
}

#endif
