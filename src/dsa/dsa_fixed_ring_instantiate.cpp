#include "dsa.hpp"

// Fixed ring batch (was DsaFixedRingBatchBase)
template class DsaEngine<FixedRingSubmitter, dsa::MutexTaskQueue>;
template class DsaEngine<FixedRingSubmitter, dsa::SingleThreadTaskQueue>;
template class DsaEngine<FixedRingSubmitter, dsa::TasSpinlockTaskQueue>;
template class DsaEngine<FixedRingSubmitter, dsa::SpinlockTaskQueue>;
template class DsaEngine<FixedRingSubmitter, dsa::BackoffSpinlockTaskQueue>;
template class DsaEngine<FixedRingSubmitter, dsa::LockFreeTaskQueue>;
