#pragma once
#ifndef DSA_IMPL_IPP
#define DSA_IMPL_IPP

#include "dsa.hpp"
#include "enum_format.hpp"
#include <atomic>
#include <cerrno>
#include <climits>
#include <cstdint>
#include <cstdlib>
#include <dsa_stdexec/error.hpp>
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

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
DsaEngine<Submitter, QueueTemplate>::DsaEngine(bool start_poller, size_t batch_size)
    : ctx_(), wq_(nullptr), wq_portal_(nullptr), task_queue_(DsaHwContext{}) {
  try {
    auto &ctx = context();
    accfg_device *device = nullptr;
    accfg_wq *wq = nullptr;
    int device_count = 0;
    int total_wq_count = 0;
    accfg_device_foreach(ctx.get(), device) {
      device_count++;
      std::string type_str = accfg_device_get_type_str(device);

      if (type_str != "dsa") {
        continue;
      }

      accfg_wq_foreach(device, wq) {
        total_wq_count++;
        auto mode = accfg_wq_get_mode(wq);
        auto state = accfg_wq_get_state(wq);

        if (accfg_wq_get_type(wq) != ACCFG_WQT_USER) {
          continue;
        }

        if (state != ACCFG_WQ_ENABLED) {
          continue;
        }

        void *portal = map_wq(wq);
        if (portal == MAP_FAILED) {
          continue;
        }

        wq_ = wq;
        wq_portal_ = portal;
        mode_ = mode;
        break;
      }

      if (wq_portal_ != nullptr) {
        break;
      }
    }
    if (device_count == 0) {
      fmt::println(stderr, "No DSA/IAX devices found. Ensure devices are enabled and "
                   "WQs configured.");
    }

    if (wq_portal_ == nullptr) {
      throw dsa_stdexec::DsaInitError(
          "Failed to locate and map a usable user work queue portal");
    }

    // Set up hardware context for the task queue
    task_queue_.hw_context().set_context(wq_portal_, mode_);

    // Initialize the descriptor submitter
    submitter_.init(wq_portal_, mode_, wq_, batch_size);

    // Provide poll callback for submitters that need backpressure support
    if constexpr (requires { submitter_.set_poll_fn(std::function<void()>{}); }) {
      submitter_.set_poll_fn([this] { task_queue_.poll(); });
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

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
DsaEngine<Submitter, QueueTemplate>::~DsaEngine() {
  running_ = false;
  if (poller_.joinable()) {
    poller_.join();
  }

  // Drain any staged/in-flight descriptors before unmapping the portal
  submitter_.drain();

  if (wq_portal_ != nullptr) {
    munmap(wq_portal_, kWqPortalSize);
    wq_portal_ = nullptr;
  }
}

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
void DsaEngine<Submitter, QueueTemplate>::data_move(void *src, void *dst, size_t size) {
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
      if (wr) { *t = *t; } else { (void)*t; }
      desc.src_addr += comp.bytes_completed;
      desc.dst_addr += comp.bytes_completed;
      desc.xfer_size -= comp.bytes_completed;
      goto retry;
    }
    throw dsa_stdexec::DsaError(status_code, comp, DSA_OPCODE_MEMMOVE,
                                "data_move");
  }
}

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
void *DsaEngine<Submitter, QueueTemplate>::map_wq(accfg_wq *wq) {
  char path[PATH_MAX] = {};
  if (accfg_wq_get_user_dev_path(wq, path, sizeof(path)) != 0) {
    fmt::println(stderr, "Failed to get user device path for WQ {}",
                 accfg_wq_get_id(wq));
    return MAP_FAILED;
  }

  int fd = open(path, O_RDWR);
  if (fd < 0) {
    std::error_code ec(errno, std::generic_category());
    fmt::println(stderr, "Failed to open {}: {}", path, ec.message());
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
    fmt::println(stderr, "Failed to mmap portal {}: {}", path,
                 mmap_error.message());
  }

  return portal;
}

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
void DsaEngine<Submitter, QueueTemplate>::submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc) {
  if (wq_portal_ == nullptr) {
    throw dsa_stdexec::DsaSubmitError("DSA work queue portal is not mapped");
  }

  if (desc != nullptr) {
    // Backpressure: wait until WQ has space before submitting
    auto cap = submitter_.wq_capacity();
    while (cap > 0 && submitter_.inflight() >= cap) {
      poll();
    }
    submitter_.submit_descriptor(desc);
  }

  task_queue_.push(op);
}

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
void DsaEngine<Submitter, QueueTemplate>::submit(dsa_stdexec::OperationBase *op) {
  task_queue_.push(op);
}

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
void DsaEngine<Submitter, QueueTemplate>::submit_raw(dsa_hw_desc *desc) {
  if (wq_portal_ == nullptr) {
    throw dsa_stdexec::DsaSubmitError("DSA work queue portal is not mapped");
  }
  if (desc == nullptr) {
    return;
  }
  _mm_sfence();
  if (mode_ == ACCFG_WQ_DEDICATED) {
    _movdir64b(wq_portal_, desc);
  } else {
    while (_enqcmd(wq_portal_, desc) != 0) {
      _mm_pause();
    }
  }
}

template <DescriptorSubmitter Submitter, template <typename> class QueueTemplate>
void DsaEngine<Submitter, QueueTemplate>::poll() {
  submitter_.pre_poll();
  auto completed = task_queue_.poll();
  submitter_.notify_complete(completed);
}

#endif
