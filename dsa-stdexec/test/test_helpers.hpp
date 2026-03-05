#pragma once
#ifndef TEST_HELPERS_HPP
#define TEST_HELPERS_HPP

#include <dsa/mock_dsa.hpp>
#include <dsa_stdexec/operation_base.hpp>

extern "C" {
#include <linux/idxd.h>
}

namespace test_helpers {

// Testable operation that inherits OperationBase and wraps a MockOperation.
// Function pointers use static_cast to recover the concrete type.
struct TestOp : dsa_stdexec::OperationBase {
  MockOperation mock;

  TestOp() {
    notify_fn = [](dsa_stdexec::OperationBase *base) {
      static_cast<TestOp *>(base)->mock.notify();
    };
    get_descriptor_fn = [](dsa_stdexec::OperationBase *base) {
      return static_cast<TestOp *>(base)->mock.get_descriptor();
    };
  }

  TestOp(const TestOp &) = delete;
  TestOp &operator=(const TestOp &) = delete;
  TestOp(TestOp &&) = delete;
  TestOp &operator=(TestOp &&) = delete;
};

} // namespace test_helpers

#endif
