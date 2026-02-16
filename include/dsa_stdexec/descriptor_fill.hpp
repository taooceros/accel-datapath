#pragma once
#ifndef DSA_STDEXEC_DESCRIPTOR_FILL_HPP
#define DSA_STDEXEC_DESCRIPTOR_FILL_HPP

#include <cstddef>
#include <cstdint>

extern "C" {
#include <linux/idxd.h>
}

namespace dsa {

// ============================================================================
// Descriptor fill functions
// ============================================================================
// Each fills opcode-specific fields. Does NOT set completion_addr
// (the caller manages that). Descriptor must be zeroed before calling.

inline void fill_data_move(dsa_hw_desc &d, void *src, void *dst, size_t size) {
  d.opcode = DSA_OPCODE_MEMMOVE;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
  d.xfer_size = static_cast<uint32_t>(size);
  d.src_addr = reinterpret_cast<uint64_t>(src);
  d.dst_addr = reinterpret_cast<uint64_t>(dst);
}

inline void fill_mem_fill(dsa_hw_desc &d, void *dst, size_t size,
                          uint64_t pattern) {
  d.opcode = DSA_OPCODE_MEMFILL;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
  d.xfer_size = static_cast<uint32_t>(size);
  d.dst_addr = reinterpret_cast<uint64_t>(dst);
  d.pattern = pattern;
}

inline void fill_compare(dsa_hw_desc &d, const void *src1, const void *src2,
                         size_t size) {
  d.opcode = DSA_OPCODE_COMPARE;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
  d.xfer_size = static_cast<uint32_t>(size);
  d.src_addr = reinterpret_cast<uint64_t>(src1);
  d.src2_addr = reinterpret_cast<uint64_t>(src2);
}

inline void fill_compare_value(dsa_hw_desc &d, const void *src, size_t size,
                               uint64_t pattern) {
  d.opcode = DSA_OPCODE_COMPVAL;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
  d.xfer_size = static_cast<uint32_t>(size);
  d.src_addr = reinterpret_cast<uint64_t>(src);
  d.comp_pattern = pattern;
}

inline void fill_dualcast(dsa_hw_desc &d, const void *src, void *dst1,
                          void *dst2, size_t size) {
  d.opcode = DSA_OPCODE_DUALCAST;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
  d.xfer_size = static_cast<uint32_t>(size);
  d.src_addr = reinterpret_cast<uint64_t>(src);
  d.dst_addr = reinterpret_cast<uint64_t>(dst1);
  d.dest2 = reinterpret_cast<uint64_t>(dst2);
}

inline void fill_crc_gen(dsa_hw_desc &d, const void *src, size_t size,
                         uint32_t seed = 0) {
  d.opcode = DSA_OPCODE_CRCGEN;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
  d.xfer_size = static_cast<uint32_t>(size);
  d.src_addr = reinterpret_cast<uint64_t>(src);
  d.crc_seed = seed;
}

inline void fill_copy_crc(dsa_hw_desc &d, const void *src, void *dst,
                          size_t size, uint32_t seed = 0) {
  d.opcode = DSA_OPCODE_COPY_CRC;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
  d.xfer_size = static_cast<uint32_t>(size);
  d.src_addr = reinterpret_cast<uint64_t>(src);
  d.dst_addr = reinterpret_cast<uint64_t>(dst);
  d.crc_seed = seed;
}

inline void fill_cache_flush(dsa_hw_desc &d, void *dst, size_t size) {
  d.opcode = DSA_OPCODE_CFLUSH;
  d.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
  d.xfer_size = static_cast<uint32_t>(size);
  d.dst_addr = reinterpret_cast<uint64_t>(dst);
}

} // namespace dsa

#endif
