#pragma once
#ifndef DSA_MIRRORED_RING_HPP
#define DSA_MIRRORED_RING_HPP

#include <cstddef>
#include <cstring>
#include <stdexcept>
#include <sys/mman.h>
#include <unistd.h>

namespace detail {
inline size_t page_align_up(size_t n) {
  long ps = sysconf(_SC_PAGESIZE);
  size_t page = static_cast<size_t>(ps > 0 ? ps : 4096);
  return (n + page - 1) & ~(page - 1);
}
} // namespace detail

#ifndef MFD_CLOEXEC
#include <linux/memfd.h>
#endif

class MirroredRing {
public:
  MirroredRing(size_t slot_count, size_t slot_size)
      : region_size_(detail::page_align_up(slot_count * slot_size)),
        map_size_(2 * region_size_) {

    // Create anonymous shared memory backing
    fd_ = memfd_create("dsa_ring", MFD_CLOEXEC);
    if (fd_ < 0) {
      throw std::runtime_error("MirroredRing: memfd_create failed");
    }

    if (ftruncate(fd_, static_cast<off_t>(region_size_)) != 0) {
      close(fd_);
      throw std::runtime_error("MirroredRing: ftruncate failed");
    }

    // Reserve 2x contiguous virtual address space
    base_ = mmap(nullptr, map_size_, PROT_NONE,
                 MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (base_ == MAP_FAILED) {
      close(fd_);
      throw std::runtime_error("MirroredRing: failed to reserve virtual address space");
    }

    // Map primary region
    void *primary = mmap(base_, region_size_, PROT_READ | PROT_WRITE,
                         MAP_SHARED | MAP_FIXED, fd_, 0);
    if (primary == MAP_FAILED) {
      munmap(base_, map_size_);
      close(fd_);
      throw std::runtime_error("MirroredRing: failed to map primary region");
    }

    // Map mirror region (same physical pages, contiguous in virtual memory)
    void *mirror = mmap(static_cast<char *>(base_) + region_size_,
                        region_size_, PROT_READ | PROT_WRITE,
                        MAP_SHARED | MAP_FIXED, fd_, 0);
    if (mirror == MAP_FAILED) {
      munmap(base_, map_size_);
      close(fd_);
      throw std::runtime_error("MirroredRing: failed to map mirror region");
    }

    // Zero-initialize
    memset(base_, 0, region_size_);
  }

  ~MirroredRing() {
    if (base_ != nullptr && base_ != MAP_FAILED) {
      munmap(base_, map_size_);
    }
    if (fd_ >= 0) {
      close(fd_);
    }
  }

  MirroredRing(const MirroredRing &) = delete;
  MirroredRing &operator=(const MirroredRing &) = delete;

  void *data() const { return base_; }
  size_t byte_size() const { return region_size_; }

private:
  void *base_ = nullptr;
  size_t region_size_ = 0;
  size_t map_size_ = 0;
  int fd_ = -1;
};

#endif
