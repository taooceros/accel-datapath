// Explicit template instantiations for DsaFixedRingBatchBase
// This file ensures the template is compiled only once

#include "dsa_fixed_ring_batch.hpp"

// Instantiate all DsaFixedRingBatch variants
template class DsaFixedRingBatchBase<dsa::MutexTaskQueue>;
template class DsaFixedRingBatchBase<dsa::SingleThreadTaskQueue>;
template class DsaFixedRingBatchBase<dsa::TasSpinlockTaskQueue>;
template class DsaFixedRingBatchBase<dsa::SpinlockTaskQueue>;
template class DsaFixedRingBatchBase<dsa::BackoffSpinlockTaskQueue>;
template class DsaFixedRingBatchBase<dsa::LockFreeTaskQueue>;
