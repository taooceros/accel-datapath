// Explicit template instantiations for DsaBase
// This file ensures the template is compiled only once

#include "dsa.hpp"

// Instantiate all Dsa variants
template class DsaBase<dsa::DefaultTaskQueue>;
template class DsaBase<dsa::SingleThreadTaskQueue>;
template class DsaBase<dsa::TasSpinlockTaskQueue>;
template class DsaBase<dsa::SpinlockTaskQueue>;
template class DsaBase<dsa::BackoffSpinlockTaskQueue>;
template class DsaBase<dsa::LockFreeTaskQueue>;
template class DsaBase<dsa::RingBufferTaskQueue1K>;
