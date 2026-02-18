#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN
#include <doctest/doctest.h>

#include <atomic>
#include <cstring>
#include <thread>
#include <vector>

// Headers under test
#include <dsa/dsa_operation_base.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/descriptor_fill.hpp>
#include <dsa_stdexec/error.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <dsa_stdexec/operations/operation_base_mixin.hpp>

extern "C" {
#include <linux/idxd.h>
}

// ============================================================================
// TEST SUITE: DsaOperationBase
// ============================================================================

TEST_SUITE("DsaOperationBase") {
  TEST_CASE("desc_ptr returns 64-byte aligned address") {
    dsa::DsaOperationBase op;
    auto addr = reinterpret_cast<uintptr_t>(op.desc_ptr());
    CHECK((addr % 64) == 0);
  }

  TEST_CASE("comp_ptr returns 32-byte aligned address") {
    dsa::DsaOperationBase op;
    auto addr = reinterpret_cast<uintptr_t>(op.comp_ptr());
    CHECK((addr % 32) == 0);
  }

  TEST_CASE("cached pointers are stable") {
    dsa::DsaOperationBase op;
    auto desc1 = op.desc_ptr();
    auto desc2 = op.desc_ptr();
    auto comp1 = op.comp_ptr();
    auto comp2 = op.comp_ptr();
    CHECK(desc1 == desc2);
    CHECK(comp1 == comp2);
  }

  TEST_CASE("multiple instances have independent aligned storage") {
    dsa::DsaOperationBase op1, op2, op3;
    auto desc1 = op1.desc_ptr();
    auto desc2 = op2.desc_ptr();
    auto desc3 = op3.desc_ptr();

    CHECK(desc1 != desc2);
    CHECK(desc2 != desc3);
    CHECK(desc1 != desc3);

    CHECK((reinterpret_cast<uintptr_t>(desc1) % 64) == 0);
    CHECK((reinterpret_cast<uintptr_t>(desc2) % 64) == 0);
    CHECK((reinterpret_cast<uintptr_t>(desc3) % 64) == 0);

    auto comp1 = op1.comp_ptr();
    auto comp2 = op2.comp_ptr();
    auto comp3 = op3.comp_ptr();

    CHECK(comp1 != comp2);
    CHECK(comp2 != comp3);
    CHECK(comp1 != comp3);

    CHECK((reinterpret_cast<uintptr_t>(comp1) % 32) == 0);
    CHECK((reinterpret_cast<uintptr_t>(comp2) % 32) == 0);
    CHECK((reinterpret_cast<uintptr_t>(comp3) % 32) == 0);
  }

  TEST_CASE("has_descriptor defaults to true") {
    dsa::DsaOperationBase op;
    CHECK(op.has_descriptor == true);
  }
}

// ============================================================================
// TEST SUITE: DescriptorFill
// ============================================================================

TEST_SUITE("DescriptorFill") {
  TEST_CASE("fill_data_move sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src[64], dst[64];
    size_t size = 1024;

    dsa::fill_data_move(desc, src, dst, size);

    CHECK(desc.opcode == DSA_OPCODE_MEMMOVE);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC));
    CHECK(desc.src_addr == reinterpret_cast<uint64_t>(src));
    CHECK(desc.dst_addr == reinterpret_cast<uint64_t>(dst));
    CHECK(desc.xfer_size == size);
  }

  TEST_CASE("fill_mem_fill sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char dst[64];
    size_t size = 2048;
    uint64_t pattern = 0xDEADBEEFCAFEBABE;

    dsa::fill_mem_fill(desc, dst, size, pattern);

    CHECK(desc.opcode == DSA_OPCODE_MEMFILL);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC));
    CHECK(desc.dst_addr == reinterpret_cast<uint64_t>(dst));
    CHECK(desc.xfer_size == size);
    CHECK(desc.pattern == pattern);
  }

  TEST_CASE("fill_compare sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src1[64], src2[64];
    size_t size = 512;

    dsa::fill_compare(desc, src1, src2, size);

    CHECK(desc.opcode == DSA_OPCODE_COMPARE);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV));
    CHECK((desc.flags & IDXD_OP_FLAG_CC) == 0);  // No cache control
    CHECK(desc.src_addr == reinterpret_cast<uint64_t>(src1));
    CHECK(desc.src2_addr == reinterpret_cast<uint64_t>(src2));
    CHECK(desc.xfer_size == size);
  }

  TEST_CASE("fill_compare_value sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src[64];
    size_t size = 128;
    uint64_t pattern = 0x1234567890ABCDEF;

    dsa::fill_compare_value(desc, src, size, pattern);

    CHECK(desc.opcode == DSA_OPCODE_COMPVAL);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV));
    CHECK(desc.src_addr == reinterpret_cast<uint64_t>(src));
    CHECK(desc.xfer_size == size);
    CHECK(desc.comp_pattern == pattern);
  }

  TEST_CASE("fill_dualcast sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src[64], dst1[64], dst2[64];
    size_t size = 256;

    dsa::fill_dualcast(desc, src, dst1, dst2, size);

    CHECK(desc.opcode == DSA_OPCODE_DUALCAST);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC));
    CHECK(desc.src_addr == reinterpret_cast<uint64_t>(src));
    CHECK(desc.dst_addr == reinterpret_cast<uint64_t>(dst1));
    CHECK(desc.dest2 == reinterpret_cast<uint64_t>(dst2));
    CHECK(desc.xfer_size == size);
  }

  TEST_CASE("fill_crc_gen sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src[64];
    size_t size = 4096;
    uint32_t seed = 0xDEADBEEF;

    dsa::fill_crc_gen(desc, src, size, seed);

    CHECK(desc.opcode == DSA_OPCODE_CRCGEN);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV));
    CHECK(desc.src_addr == reinterpret_cast<uint64_t>(src));
    CHECK(desc.xfer_size == size);
    CHECK(desc.crc_seed == seed);
  }

  TEST_CASE("fill_crc_gen with default seed") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src[64];
    size_t size = 1024;

    dsa::fill_crc_gen(desc, src, size);

    CHECK(desc.opcode == DSA_OPCODE_CRCGEN);
    CHECK(desc.crc_seed == 0);
  }

  TEST_CASE("fill_copy_crc sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src[64], dst[64];
    size_t size = 8192;
    uint32_t seed = 0xCAFEBABE;

    dsa::fill_copy_crc(desc, src, dst, size, seed);

    CHECK(desc.opcode == DSA_OPCODE_COPY_CRC);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV));
    CHECK(desc.src_addr == reinterpret_cast<uint64_t>(src));
    CHECK(desc.dst_addr == reinterpret_cast<uint64_t>(dst));
    CHECK(desc.xfer_size == size);
    CHECK(desc.crc_seed == seed);
  }

  TEST_CASE("fill_copy_crc with default seed") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char src[64], dst[64];
    size_t size = 1024;

    dsa::fill_copy_crc(desc, src, dst, size);

    CHECK(desc.opcode == DSA_OPCODE_COPY_CRC);
    CHECK(desc.crc_seed == 0);
  }

  TEST_CASE("fill_cache_flush sets correct fields") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));

    char dst[64];
    size_t size = 16384;

    dsa::fill_cache_flush(desc, dst, size);

    CHECK(desc.opcode == DSA_OPCODE_CFLUSH);
    CHECK(desc.flags == (IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV));
    CHECK(desc.dst_addr == reinterpret_cast<uint64_t>(dst));
    CHECK(desc.xfer_size == size);
  }
}

// ============================================================================
// TEST SUITE: DsaError
// ============================================================================

TEST_SUITE("DsaError") {
  TEST_CASE("dsa_status_to_string converts status codes correctly") {
    CHECK(dsa_stdexec::dsa_status_to_string(DSA_COMP_SUCCESS) == "Success");
    CHECK(dsa_stdexec::dsa_status_to_string(DSA_COMP_NONE) == "No status (operation not complete)");
    CHECK(dsa_stdexec::dsa_status_to_string(DSA_COMP_PAGE_FAULT_NOBOF) == "Page fault without block-on-fault");
    CHECK(dsa_stdexec::dsa_status_to_string(DSA_COMP_BAD_OPCODE) == "Invalid opcode");
    CHECK(dsa_stdexec::dsa_status_to_string(0xFF) == "Unknown error");
  }

  TEST_CASE("dsa_opcode_to_string converts opcodes correctly") {
    CHECK(dsa_stdexec::dsa_opcode_to_string(DSA_OPCODE_MEMMOVE) == "MEMMOVE");
    CHECK(dsa_stdexec::dsa_opcode_to_string(DSA_OPCODE_MEMFILL) == "MEMFILL");
    CHECK(dsa_stdexec::dsa_opcode_to_string(DSA_OPCODE_BATCH) == "BATCH");
    CHECK(dsa_stdexec::dsa_opcode_to_string(DSA_OPCODE_COMPARE) == "COMPARE");
    CHECK(dsa_stdexec::dsa_opcode_to_string(DSA_OPCODE_CRCGEN) == "CRCGEN");
    CHECK(dsa_stdexec::dsa_opcode_to_string(0xFF) == "UNKNOWN");
  }

  TEST_CASE("DsaError construction with status and completion record") {
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));
    comp.status = DSA_COMP_BAD_OPCODE;
    comp.bytes_completed = 512;
    comp.fault_addr = 0xDEADBEEF00000000;
    comp.fault_info = 0x42;

    dsa_stdexec::DsaError err(DSA_COMP_BAD_OPCODE, comp, DSA_OPCODE_MEMMOVE, "test context");

    CHECK(err.status() == DSA_COMP_BAD_OPCODE);
    CHECK(err.opcode() == DSA_OPCODE_MEMMOVE);
    CHECK(err.bytes_completed() == 512);
    CHECK(err.fault_addr() == 0xDEADBEEF00000000);
    CHECK(err.fault_info() == 0x42);
    CHECK(err.status_string() == "Invalid opcode");
    CHECK(err.opcode_string() == "MEMMOVE");

    std::string what_str = err.what();
    CHECK(what_str.find("test context") != std::string::npos);
    CHECK(what_str.find("Invalid opcode") != std::string::npos);
  }

  TEST_CASE("DsaError construction with message string") {
    dsa_stdexec::DsaError err("custom error message");

    std::string what_str = err.what();
    CHECK(what_str.find("custom error message") != std::string::npos);
    CHECK(err.status() == 0);
    CHECK(err.opcode() == 0);
  }

  TEST_CASE("DsaError construction with status code only") {
    dsa_stdexec::DsaError err(DSA_COMP_PAGE_FAULT_NOBOF);

    CHECK(err.status() == DSA_COMP_PAGE_FAULT_NOBOF);
    CHECK(err.status_string() == "Page fault without block-on-fault");
  }

  TEST_CASE("DsaSubmitError with reason") {
    dsa_stdexec::DsaSubmitError err("queue full");

    std::string what_str = err.what();
    CHECK(what_str.find("Failed to submit DSA descriptor") != std::string::npos);
    CHECK(what_str.find("queue full") != std::string::npos);
    CHECK(err.error_code() == 0);
  }

  TEST_CASE("DsaSubmitError with reason and error code") {
    dsa_stdexec::DsaSubmitError err("enqcmd failed", EAGAIN);

    std::string what_str = err.what();
    CHECK(what_str.find("enqcmd failed") != std::string::npos);
    CHECK(what_str.find("errno") != std::string::npos);
    CHECK(err.error_code() == EAGAIN);
  }

  TEST_CASE("DsaInitError with reason") {
    dsa_stdexec::DsaInitError err("no devices found");

    std::string what_str = err.what();
    CHECK(what_str.find("DSA initialization failed") != std::string::npos);
    CHECK(what_str.find("no devices found") != std::string::npos);
    CHECK(err.error_code() == 0);
  }

  TEST_CASE("DsaInitError with reason and error code") {
    dsa_stdexec::DsaInitError err("mmap failed", ENOMEM);

    std::string what_str = err.what();
    CHECK(what_str.find("mmap failed") != std::string::npos);
    CHECK(what_str.find("errno") != std::string::npos);
    CHECK(err.error_code() == ENOMEM);
  }

  TEST_CASE("DsaError inherits from std::exception") {
    dsa_stdexec::DsaError err("test");
    std::exception& base_ref = err;
    CHECK(base_ref.what() != nullptr);

    // Can catch as std::exception
    bool caught = false;
    try {
      throw dsa_stdexec::DsaError("test exception");
    } catch (const std::exception& e) {
      caught = true;
    }
    CHECK(caught);
  }
}

// ============================================================================
// TEST SUITE: OperationBase
// ============================================================================

TEST_SUITE("OperationBase") {
  // Test operation that inherits from OperationBase for function pointer dispatch
  struct TestOp : dsa_stdexec::OperationBase {
    int notify_count = 0;
    dsa_hw_desc* desc = nullptr;

    TestOp() {
      notify_fn = [](dsa_stdexec::OperationBase *base) {
        static_cast<TestOp *>(base)->notify_count++;
      };
      get_descriptor_fn = [](dsa_stdexec::OperationBase *base) {
        return static_cast<TestOp *>(base)->desc;
      };
    }
  };

  TEST_CASE("function pointer dispatch to notify") {
    TestOp op;
    CHECK(op.notify_count == 0);
    op.notify();
    CHECK(op.notify_count == 1);
    op.notify();
    CHECK(op.notify_count == 2);
  }

  TEST_CASE("function pointer dispatch to get_descriptor") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    desc.opcode = DSA_OPCODE_MEMMOVE;

    TestOp op;
    op.desc = &desc;

    dsa_hw_desc* returned = op.get_descriptor();
    CHECK(returned == &desc);
    CHECK(returned->opcode == DSA_OPCODE_MEMMOVE);
  }

  TEST_CASE("next pointer defaults to nullptr") {
    dsa_stdexec::OperationBase op1, op2;
    CHECK(op1.next == nullptr);
    CHECK(op2.next == nullptr);
  }

  TEST_CASE("next pointer can link operations") {
    dsa_stdexec::OperationBase op1, op2, op3;

    op1.next = &op2;
    op2.next = &op3;
    op3.next = nullptr;

    CHECK(op1.next == &op2);
    CHECK(op2.next == &op3);
    CHECK(op3.next == nullptr);

    // Can traverse the list
    auto* current = &op1;
    int count = 0;
    while (current != nullptr) {
      ++count;
      current = current->next;
    }
    CHECK(count == 3);
  }
}

// ============================================================================
// TEST SUITE: PageFaultAdjustment
// ============================================================================

TEST_SUITE("PageFaultAdjustment") {
  TEST_CASE("adjust_for_page_fault for MEMMOVE") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 256;

    desc.opcode = DSA_OPCODE_MEMMOVE;
    desc.src_addr = 0x1000;
    desc.dst_addr = 0x2000;
    desc.xfer_size = 1024;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.src_addr == 0x1000 + 256);
    CHECK(desc.dst_addr == 0x2000 + 256);
    CHECK(desc.xfer_size == 1024 - 256);
  }

  TEST_CASE("adjust_for_page_fault for MEMFILL") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 512;

    desc.opcode = DSA_OPCODE_MEMFILL;
    desc.dst_addr = 0x3000;
    desc.xfer_size = 2048;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.dst_addr == 0x3000 + 512);
    CHECK(desc.xfer_size == 2048 - 512);
  }

  TEST_CASE("adjust_for_page_fault for COMPARE") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 128;

    desc.opcode = DSA_OPCODE_COMPARE;
    desc.src_addr = 0x4000;
    desc.src2_addr = 0x5000;
    desc.xfer_size = 1024;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.src_addr == 0x4000 + 128);
    CHECK(desc.src2_addr == 0x5000 + 128);
    CHECK(desc.xfer_size == 1024 - 128);
  }

  TEST_CASE("adjust_for_page_fault for COMPVAL") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 64;

    desc.opcode = DSA_OPCODE_COMPVAL;
    desc.src_addr = 0x6000;
    desc.xfer_size = 512;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.src_addr == 0x6000 + 64);
    CHECK(desc.xfer_size == 512 - 64);
  }

  TEST_CASE("adjust_for_page_fault for DUALCAST") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 1024;

    desc.opcode = DSA_OPCODE_DUALCAST;
    desc.src_addr = 0x7000;
    desc.dst_addr = 0x8000;
    desc.dest2 = 0x9000;
    desc.xfer_size = 4096;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.src_addr == 0x7000 + 1024);
    CHECK(desc.dst_addr == 0x8000 + 1024);
    CHECK(desc.dest2 == 0x9000 + 1024);
    CHECK(desc.xfer_size == 4096 - 1024);
  }

  TEST_CASE("adjust_for_page_fault for CRCGEN") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 2048;
    comp.crc_val = 0xABCD1234;

    desc.opcode = DSA_OPCODE_CRCGEN;
    desc.src_addr = 0xA000;
    desc.xfer_size = 8192;
    desc.crc_seed = 0;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.crc_seed == 0xABCD1234);
    CHECK(desc.src_addr == 0xA000 + 2048);
    CHECK(desc.xfer_size == 8192 - 2048);
  }

  TEST_CASE("adjust_for_page_fault for COPY_CRC") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 4096;
    comp.crc_val = 0xDEADBEEF;

    desc.opcode = DSA_OPCODE_COPY_CRC;
    desc.src_addr = 0xB000;
    desc.dst_addr = 0xC000;
    desc.xfer_size = 16384;
    desc.crc_seed = 0;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.crc_seed == 0xDEADBEEF);
    CHECK(desc.src_addr == 0xB000 + 4096);
    CHECK(desc.dst_addr == 0xC000 + 4096);
    CHECK(desc.xfer_size == 16384 - 4096);
  }

  TEST_CASE("adjust_for_page_fault for CFLUSH") {
    dsa_hw_desc desc;
    std::memset(&desc, 0, sizeof(desc));
    dsa_completion_record comp;
    std::memset(&comp, 0, sizeof(comp));

    char dummy_page[64] = {};
    comp.fault_addr = reinterpret_cast<uint64_t>(dummy_page);
    comp.status = DSA_COMP_PAGE_FAULT_NOBOF;
    comp.bytes_completed = 256;

    desc.opcode = DSA_OPCODE_CFLUSH;
    desc.dst_addr = 0xD000;
    desc.xfer_size = 1024;

    dsa_stdexec::adjust_for_page_fault(desc, comp);

    CHECK(desc.dst_addr == 0xD000 + 256);
    CHECK(desc.xfer_size == 1024 - 256);
  }

  TEST_CASE("page fault retry counter") {
    dsa_stdexec::reset_page_fault_retries();
    CHECK(dsa_stdexec::get_page_fault_retries() == 0);

    dsa_stdexec::g_page_fault_retries.fetch_add(1, std::memory_order_relaxed);
    CHECK(dsa_stdexec::get_page_fault_retries() == 1);

    dsa_stdexec::g_page_fault_retries.fetch_add(5, std::memory_order_relaxed);
    CHECK(dsa_stdexec::get_page_fault_retries() == 6);

    dsa_stdexec::reset_page_fault_retries();
    CHECK(dsa_stdexec::get_page_fault_retries() == 0);
  }
}

// ============================================================================
// TEST SUITE: Locks
// ============================================================================

TEST_SUITE("Locks") {
  TEST_CASE("NullLock is no-op") {
    dsa::locks::NullLock lock;
    lock.lock();
    lock.unlock();
    lock.lock();
    lock.lock();
    lock.unlock();
    lock.unlock();
    // No assertions needed - just verify it compiles and doesn't crash
    CHECK(true);
  }

  TEST_CASE("NullLock satisfies Lockable concept") {
    CHECK(dsa::Lockable<dsa::locks::NullLock>);
  }

  TEST_CASE("MutexLock lock and unlock") {
    dsa::locks::MutexLock lock;
    lock.lock();
    lock.unlock();
    CHECK(true);
  }

  TEST_CASE("MutexLock satisfies Lockable concept") {
    CHECK(dsa::Lockable<dsa::locks::MutexLock>);
  }

  TEST_CASE("TasSpinlock lock and unlock") {
    dsa::locks::TasSpinlock lock;
    lock.lock();
    lock.unlock();
    lock.lock();
    lock.unlock();
    CHECK(true);
  }

  TEST_CASE("TasSpinlock satisfies Lockable concept") {
    CHECK(dsa::Lockable<dsa::locks::TasSpinlock>);
  }

  TEST_CASE("TtasSpinlock lock and unlock") {
    dsa::locks::TtasSpinlock lock;
    lock.lock();
    lock.unlock();
    lock.lock();
    lock.unlock();
    CHECK(true);
  }

  TEST_CASE("TtasSpinlock satisfies Lockable concept") {
    CHECK(dsa::Lockable<dsa::locks::TtasSpinlock>);
  }

  TEST_CASE("TtasBackoffSpinlock lock and unlock") {
    dsa::locks::TtasBackoffSpinlock lock;
    lock.lock();
    lock.unlock();
    lock.lock();
    lock.unlock();
    CHECK(true);
  }

  TEST_CASE("TtasBackoffSpinlock satisfies Lockable concept") {
    CHECK(dsa::Lockable<dsa::locks::TtasBackoffSpinlock>);
  }

  TEST_CASE("MutexLock concurrent access protection") {
    dsa::locks::MutexLock lock;
    std::atomic<int> counter{0};
    constexpr int num_threads = 4;
    constexpr int increments_per_thread = 1000;

    std::vector<std::thread> threads;
    threads.reserve(num_threads);

    for (int i = 0; i < num_threads; ++i) {
      threads.emplace_back([&]() {
        for (int j = 0; j < increments_per_thread; ++j) {
          lock.lock();
          int temp = counter.load(std::memory_order_relaxed);
          temp++;
          counter.store(temp, std::memory_order_relaxed);
          lock.unlock();
        }
      });
    }

    for (auto& t : threads) {
      t.join();
    }

    CHECK(counter.load() == num_threads * increments_per_thread);
  }

  TEST_CASE("TtasSpinlock concurrent access protection") {
    dsa::locks::TtasSpinlock lock;
    std::atomic<int> counter{0};
    constexpr int num_threads = 4;
    constexpr int increments_per_thread = 1000;

    std::vector<std::thread> threads;
    threads.reserve(num_threads);

    for (int i = 0; i < num_threads; ++i) {
      threads.emplace_back([&]() {
        for (int j = 0; j < increments_per_thread; ++j) {
          lock.lock();
          int temp = counter.load(std::memory_order_relaxed);
          temp++;
          counter.store(temp, std::memory_order_relaxed);
          lock.unlock();
        }
      });
    }

    for (auto& t : threads) {
      t.join();
    }

    CHECK(counter.load() == num_threads * increments_per_thread);
  }

  TEST_CASE("TtasBackoffSpinlock concurrent access protection") {
    dsa::locks::TtasBackoffSpinlock lock;
    std::atomic<int> counter{0};
    constexpr int num_threads = 4;
    constexpr int increments_per_thread = 1000;

    std::vector<std::thread> threads;
    threads.reserve(num_threads);

    for (int i = 0; i < num_threads; ++i) {
      threads.emplace_back([&]() {
        for (int j = 0; j < increments_per_thread; ++j) {
          lock.lock();
          int temp = counter.load(std::memory_order_relaxed);
          temp++;
          counter.store(temp, std::memory_order_relaxed);
          lock.unlock();
        }
      });
    }

    for (auto& t : threads) {
      t.join();
    }

    CHECK(counter.load() == num_threads * increments_per_thread);
  }

  TEST_CASE("TasSpinlock concurrent access protection") {
    dsa::locks::TasSpinlock lock;
    std::atomic<int> counter{0};
    constexpr int num_threads = 4;
    constexpr int increments_per_thread = 1000;

    std::vector<std::thread> threads;
    threads.reserve(num_threads);

    for (int i = 0; i < num_threads; ++i) {
      threads.emplace_back([&]() {
        for (int j = 0; j < increments_per_thread; ++j) {
          lock.lock();
          int temp = counter.load(std::memory_order_relaxed);
          temp++;
          counter.store(temp, std::memory_order_relaxed);
          lock.unlock();
        }
      });
    }

    for (auto& t : threads) {
      t.join();
    }

    CHECK(counter.load() == num_threads * increments_per_thread);
  }
}
