#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN
#include <doctest/doctest.h>

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <numeric>
#include <string>
#include <vector>

#include <dsa/dsa.hpp>
#include <dsa/mirrored_ring.hpp>
#include <dsa_stdexec/batch.hpp>
#include <dsa_stdexec/descriptor_fill.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>

// ============================================================================
// Test 1: MirroredRing memory aliasing (no hardware needed)
// ============================================================================

TEST_CASE("MirroredRing memory aliasing") {
  constexpr size_t kSlots = 256;
  constexpr size_t kSlotSize = 64; // sizeof(dsa_hw_desc)
  MirroredRing ring(kSlots, kSlotSize);

  auto *primary = static_cast<uint8_t *>(ring.data());
  auto *mirror = primary + ring.byte_size(); // starts right after primary

  REQUIRE(ring.byte_size() == kSlots * kSlotSize);

  SUBCASE("write primary, read mirror") {
    for (size_t i = 0; i < kSlots * kSlotSize; i++) {
      primary[i] = static_cast<uint8_t>(i & 0xFF);
    }
    for (size_t i = 0; i < kSlots * kSlotSize; i++) {
      CHECK(mirror[i] == static_cast<uint8_t>(i & 0xFF));
    }
  }

  SUBCASE("write mirror, read primary") {
    for (size_t i = 0; i < kSlots * kSlotSize; i++) {
      mirror[i] = static_cast<uint8_t>((i * 7 + 13) & 0xFF);
    }
    for (size_t i = 0; i < kSlots * kSlotSize; i++) {
      CHECK(primary[i] == static_cast<uint8_t>((i * 7 + 13) & 0xFF));
    }
  }

  SUBCASE("contiguous access across boundary") {
    // Write to slots 250..261 using masked indices (as the submitter would)
    auto *slots = reinterpret_cast<dsa_hw_desc *>(primary);
    for (size_t i = 250; i < 262; i++) {
      size_t idx = i & (kSlots - 1); // mask to [0, 256)
      slots[idx].src_addr = static_cast<uint64_t>(i);
    }

    // Read contiguously from &slots[250] — slots 256..261 are in the mirror
    // region but appear contiguous in virtual memory
    dsa_hw_desc *cross = &slots[250];
    for (size_t i = 0; i < 12; i++) {
      CHECK(cross[i].src_addr == static_cast<uint64_t>(250 + i));
    }
  }
}

// ============================================================================
// Test 2: Basic data_move through MirroredRingSubmitter
// ============================================================================

TEST_CASE("basic data_move through MirroredRingSubmitter") {
  DsaMirroredRingBatchSingleThread dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  std::string src = "Hello, MirroredRing!";
  std::string dst(src.size(), '\0');

  auto sender = dsa_stdexec::dsa_data_move(dsa, src.data(), dst.data(), src.size());
  dsa_stdexec::wait_start(std::move(sender), loop);

  CHECK(dst == src);
}

// ============================================================================
// Test 3: Cross-boundary batch (the critical test)
// ============================================================================

TEST_CASE("cross-boundary batch") {
  DsaMirroredRingBatchSingleThread dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Step 1: Submit 250 individual data_move ops to advance desc_tail_ past 250.
  // Each op copies 64 bytes. We poll after each to reclaim ring slots.
  constexpr size_t kWarmupOps = 250;
  constexpr size_t kChunkSize = 64;
  std::vector<uint8_t> warmup_src(kWarmupOps * kChunkSize);
  std::vector<uint8_t> warmup_dst(kWarmupOps * kChunkSize, 0);
  std::iota(warmup_src.begin(), warmup_src.end(), uint8_t{1});

  for (size_t i = 0; i < kWarmupOps; i++) {
    auto sender = dsa_stdexec::dsa_data_move(
        dsa, warmup_src.data() + i * kChunkSize,
        warmup_dst.data() + i * kChunkSize, kChunkSize);
    dsa_stdexec::wait_start(std::move(sender), loop);
    loop.reset();
  }

  // Verify warmup ops completed correctly
  CHECK(warmup_src == warmup_dst);

  // Step 2: Submit a batch of 32 ops that spans the ring boundary.
  // desc_tail_ is now at ~250, so 32 descriptors span slots 250..281,
  // crossing the 256-slot boundary. With mirroring, hardware DMA-reads
  // contiguous memory through the mirror region.
  constexpr size_t kBatchSize = 32;
  std::vector<uint8_t> batch_src(kBatchSize * kChunkSize);
  std::vector<uint8_t> batch_dst(kBatchSize * kChunkSize, 0);
  for (size_t i = 0; i < batch_src.size(); i++) {
    batch_src[i] = static_cast<uint8_t>((i * 3 + 7) & 0xFF);
  }

  auto sender = dsa_stdexec::dsa_batch(
      dsa, kBatchSize, [&](std::span<dsa_hw_desc> descs) {
        for (size_t i = 0; i < kBatchSize; i++) {
          dsa::fill_data_move(descs[i], batch_src.data() + i * kChunkSize,
                              batch_dst.data() + i * kChunkSize, kChunkSize);
        }
      });
  dsa_stdexec::wait_start(std::move(sender), loop);

  CHECK(batch_src == batch_dst);
}

// ============================================================================
// Test 4: Sustained ring cycling (stress)
// ============================================================================

TEST_CASE("sustained ring cycling") {
  DsaMirroredRingBatchSingleThread dsa(false);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  // Run 2000 data_move ops in batches of 32 via dsa_batch.
  // Ring has 256 slots, so 2000 ops cycle through it ~8 times,
  // hitting every possible boundary alignment.
  constexpr size_t kTotalOps = 2000;
  constexpr size_t kBatchSize = 32;
  constexpr size_t kChunkSize = 64;

  std::vector<uint8_t> src(kTotalOps * kChunkSize);
  std::vector<uint8_t> dst(kTotalOps * kChunkSize, 0);
  for (size_t i = 0; i < src.size(); i++) {
    src[i] = static_cast<uint8_t>((i * 11 + 3) & 0xFF);
  }

  size_t op_idx = 0;
  while (op_idx < kTotalOps) {
    size_t batch = std::min(kBatchSize, kTotalOps - op_idx);

    auto sender = dsa_stdexec::dsa_batch(
        dsa, batch, [&](std::span<dsa_hw_desc> descs) {
          for (size_t i = 0; i < descs.size(); i++) {
            size_t offset = (op_idx + i) * kChunkSize;
            dsa::fill_data_move(descs[i], src.data() + offset,
                                dst.data() + offset, kChunkSize);
          }
        });
    dsa_stdexec::wait_start(std::move(sender), loop);
    loop.reset();
    op_idx += batch;
  }

  CHECK(src == dst);
}
