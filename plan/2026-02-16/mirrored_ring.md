# MirroredRingSubmitter: Virtual-Memory Wrap-Free Ring Buffer

## Context

The current `RingSubmitter` in `src/dsa/descriptor_submitter.hpp` uses a 256-slot
descriptor ring (16KB = 4 pages). When a batch of descriptors would cross the ring
boundary, `submit_descriptor()` must force-seal the current batch and start a new
one at index 0:

```cpp
size_t next_idx = desc_index(desc_tail_);
if (batch.count > 0 && next_idx == 0) {
    seal_and_submit_current();  // forces a short batch
    // ... set up new batch at index 0
}
```

This creates unnecessarily small batches at the wrap point, increasing MMIO doorbell
writes and reducing batching efficiency.

## Goal

Add a new `MirroredRingSubmitter` that uses a virtual memory trick to eliminate
wrap-around handling entirely. The descriptor ring is backed by a shared memory
file (via `memfd_create`), with the same physical pages mapped twice contiguously
in virtual memory:

```
Virtual address space:   [ page0 page1 page2 page3 | page0 page1 page2 page3 ]
Physical pages (memfd):  [ page0 page1 page2 page3 ]
                          ^--- primary region ---^   ^--- mirror region ----^
```

With this layout, `&ring[idx]` for any `idx` in `[0, 2*N)` is always valid, and
a batch starting at index 250 with 32 descriptors sees slots 250..281 as
contiguous memory — even though slots 256..281 are physically slots 0..25.
Hardware DMA reads the correct physical pages via IOMMU translation.

This is a **new submitter**, not a replacement for `RingSubmitter`. Both will
coexist — the benchmark can compare them.

## Design

### `MirroredRing` — RAII wrapper for the mmap'd ring

```cpp
class MirroredRing {
public:
    MirroredRing(size_t slot_count, size_t slot_size);
    ~MirroredRing();

    void *data() const;              // base pointer to primary region
    size_t byte_size() const;        // total bytes of primary region

    MirroredRing(const MirroredRing &) = delete;
    MirroredRing &operator=(const MirroredRing &) = delete;

private:
    void *base_ = nullptr;           // mmap'd base (2x region)
    size_t map_size_ = 0;            // total virtual mapping size (2x)
    int fd_ = -1;                    // memfd file descriptor
};
```

Constructor:
1. `memfd_create("dsa_ring", MFD_CLOEXEC)` — create anonymous shared memory
2. `ftruncate(fd, byte_size)` — set size to `slot_count * slot_size`
3. `mmap(NULL, 2 * byte_size, PROT_NONE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0)` —
   reserve 2x virtual address space
4. `mmap(base, byte_size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED, fd, 0)` —
   map primary region
5. `mmap(base + byte_size, byte_size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED, fd, 0)` —
   map mirror region at the same fd offset

Destructor: `munmap(base_, 2 * byte_size)`, `close(fd_)`.

### `MirroredRingSubmitter`

Same structure as `RingSubmitter` but:
- `desc_ring_` is a `MirroredRing` instead of `alignas(64) dsa_hw_desc[N]`
- No wrap-around check in `submit_descriptor()` — removed entirely
- `batch.start` can point into the mirror region; `desc_list_addr` just uses
  the pointer directly — hardware sees contiguous physical pages

Key difference in `submit_descriptor()` — no wrap check needed because the
mirror region makes the ring contiguous across the boundary.

Key difference in `submit_batch()` — `desc_list_addr` uses the raw start position
which may point into the mirror region, where the IOMMU resolves both virtual
addresses to the same physical memory.

## Files

| File | Action | Description |
|------|--------|-------------|
| `src/dsa/mirrored_ring.hpp` | **CREATE** | `MirroredRing` RAII class |
| `src/dsa/descriptor_submitter.hpp` | MODIFY | Add `MirroredRingSubmitter` |
| `src/dsa/dsa.hpp` | MODIFY | Add `DsaMirroredRingBatch*` type aliases |
| `src/dsa/dsa_mirrored_ring_instantiate.cpp` | **CREATE** | Explicit instantiations |
| `xmake.lua` | MODIFY | Add new instantiate file to `libdsa` |

## Implementation Order

1. Create `src/dsa/mirrored_ring.hpp` with `MirroredRing` class
2. Add `MirroredRingSubmitter` to `descriptor_submitter.hpp`
3. Add type aliases + instantiation file
4. Update `xmake.lua`
5. Build + test + benchmark against `RingSubmitter`

## Risk Notes

- **IOMMU compatibility**: DSA uses IOMMU for address translation. Two virtual
  pages mapping the same physical page is standard mmap behavior — the IOMMU
  resolves both virtual addresses to the same physical memory. No special
  handling needed.
- **memfd_create availability**: Available since Linux 3.17 (2014). Not a concern
  for DSA-capable hardware which requires Linux 5.10+.
- **Memory overhead**: 4 extra pages of virtual address space (not physical).
  The physical footprint is identical to `RingSubmitter`.
