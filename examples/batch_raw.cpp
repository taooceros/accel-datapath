// Example: Raw DSA Batch Descriptor
// Submits multiple data move operations as a single hardware batch descriptor
// (opcode 0x01) directly, bypassing the sender/receiver abstraction.

#include <cstdint>
#include <cstring>
#include <fmt/base.h>
#include <string>
#include <vector>
#include <x86intrin.h>

extern "C" {
#include <accel-config/libaccel_config.h>
#include <linux/idxd.h>
}

#include <dsa/dsa.hpp>

int main() {
  // Create DSA instance (no poller thread — we poll synchronously)
  Dsa dsa(false);

  // --- Prepare sub-descriptors (3 independent data_move ops) ---
  constexpr int kBatchSize = 3;

  std::vector<std::string> srcs = {
      "Hello from batch op 0!",
      "Hello from batch op 1!",
      "Hello from batch op 2!",
  };
  std::vector<std::string> dsts(kBatchSize);
  for (int i = 0; i < kBatchSize; ++i) {
    dsts[i].resize(srcs[i].size(), '\0');
  }

  // Sub-descriptors must be in a contiguous 64-byte-aligned array.
  // Each sub-descriptor has its own completion record for individual status.
  alignas(64) dsa_hw_desc sub_descs[kBatchSize] = {};
  alignas(32) dsa_completion_record sub_comps[kBatchSize] = {};

  for (int i = 0; i < kBatchSize; ++i) {
    memset(&sub_descs[i], 0, sizeof(dsa_hw_desc));
    memset(&sub_comps[i], 0, sizeof(dsa_completion_record));

    sub_descs[i].opcode = DSA_OPCODE_MEMMOVE;
    sub_descs[i].flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    sub_descs[i].xfer_size = static_cast<uint32_t>(srcs[i].size());
    sub_descs[i].src_addr = reinterpret_cast<uint64_t>(srcs[i].data());
    sub_descs[i].dst_addr = reinterpret_cast<uint64_t>(dsts[i].data());
    sub_descs[i].completion_addr = reinterpret_cast<uint64_t>(&sub_comps[i]);
  }

  // --- Build the batch descriptor ---
  alignas(64) dsa_hw_desc batch_desc = {};
  alignas(32) dsa_completion_record batch_comp = {};
  memset(&batch_desc, 0, sizeof(batch_desc));
  memset(&batch_comp, 0, sizeof(batch_comp));

  batch_desc.opcode = DSA_OPCODE_BATCH;
  batch_desc.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
  batch_desc.desc_list_addr = reinterpret_cast<uint64_t>(&sub_descs[0]);
  batch_desc.desc_count = kBatchSize;
  batch_desc.completion_addr = reinterpret_cast<uint64_t>(&batch_comp);

  fmt::println("Submitting batch of {} data_move operations...", kBatchSize);

  // --- Submit via submit_raw (single MMIO doorbell write) ---
  dsa.submit_raw(&batch_desc);

  // --- Spin-wait for batch completion ---
  while (batch_comp.status == 0) {
    _mm_pause();
  }

  uint8_t batch_status = batch_comp.status & DSA_COMP_STATUS_MASK;
  if (batch_status != DSA_COMP_SUCCESS) {
    fmt::println(stderr, "Batch failed with status {:#x}", batch_status);
    return 1;
  }

  fmt::println("Batch completed successfully!");

  // --- Verify each sub-operation ---
  for (int i = 0; i < kBatchSize; ++i) {
    uint8_t sub_status = sub_comps[i].status & DSA_COMP_STATUS_MASK;
    if (sub_status != DSA_COMP_SUCCESS) {
      fmt::println(stderr, "  Sub-op {} failed with status {:#x}", i, sub_status);
      return 1;
    }
    fmt::println("  Op {}: \"{}\" -> \"{}\"", i, srcs[i], dsts[i]);
  }

  return 0;
}
