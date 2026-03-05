#pragma once
#ifndef DSA_STDEXEC_DSA_SINK_HPP
#define DSA_STDEXEC_DSA_SINK_HPP

#include <concepts>
#include <dsa_stdexec/operation_base.hpp>

extern "C" {
#include <linux/idxd.h>
}

namespace dsa_stdexec {

// Concept formalizing the descriptor sink interface that DsaBase, DsaBatchBase,
// DsaRingBatchBase, and DsaFixedRingBatchBase all satisfy.
template <typename T>
concept DsaSink = requires(T &dsa, OperationBase *op, dsa_hw_desc *desc) {
    { dsa.submit(op, desc) } -> std::same_as<void>;
    { dsa.submit(op) } -> std::same_as<void>;
    { dsa.poll() };
};

} // namespace dsa_stdexec

#endif
