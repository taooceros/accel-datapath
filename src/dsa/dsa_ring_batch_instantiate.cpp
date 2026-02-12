// Explicit template instantiations for DsaRingBatchBase
// This file ensures the template is compiled only once

#include "dsa_ring_batch.hpp"

// Instantiate all DsaRingBatch variants
template class DsaRingBatchBase<dsa::MutexTaskQueue>;
template class DsaRingBatchBase<dsa::SingleThreadTaskQueue>;
template class DsaRingBatchBase<dsa::TasSpinlockTaskQueue>;
template class DsaRingBatchBase<dsa::SpinlockTaskQueue>;
template class DsaRingBatchBase<dsa::BackoffSpinlockTaskQueue>;
template class DsaRingBatchBase<dsa::LockFreeTaskQueue>;
