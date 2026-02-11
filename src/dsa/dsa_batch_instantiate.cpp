// Explicit template instantiations for DsaBatchBase
// This file ensures the template is compiled only once

#include "dsa_batch.hpp"

// Instantiate all DsaBatch variants
template class DsaBatchBase<dsa::MutexTaskQueue>;
template class DsaBatchBase<dsa::SingleThreadTaskQueue>;
template class DsaBatchBase<dsa::TasSpinlockTaskQueue>;
template class DsaBatchBase<dsa::SpinlockTaskQueue>;
template class DsaBatchBase<dsa::BackoffSpinlockTaskQueue>;
template class DsaBatchBase<dsa::LockFreeTaskQueue>;
