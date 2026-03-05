#pragma once
#ifndef DSA_STDEXEC_OPERATIONS_ALL_HPP
#define DSA_STDEXEC_OPERATIONS_ALL_HPP

// Unified header for all DSA operation senders
//
// This header includes all available DSA operation senders:
// - MemFillSender: Fill memory with a 64-bit pattern
// - CompareSender: Compare two memory regions
// - CompareValueSender: Compare memory against a 64-bit pattern
// - DualcastSender: Copy to two destinations simultaneously
// - CrcGenSender: Generate CRC-32C checksum
// - CopyCrcSender: Copy with CRC-32C generation
// - CacheFlushSender: Flush CPU cache lines
// - DataMoveSender: Copy memory from source to destination

#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/operations/mem_fill.hpp>
#include <dsa_stdexec/operations/compare.hpp>
#include <dsa_stdexec/operations/compare_value.hpp>
#include <dsa_stdexec/operations/dualcast.hpp>
#include <dsa_stdexec/operations/crc_gen.hpp>
#include <dsa_stdexec/operations/copy_crc.hpp>
#include <dsa_stdexec/operations/cache_flush.hpp>

#endif
