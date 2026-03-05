#include "dsa.hpp"

// Ring batch (was DsaRingBatchBase)
template class DsaEngine<RingSubmitter, dsa::MutexTaskQueue>;
template class DsaEngine<RingSubmitter, dsa::SingleThreadTaskQueue>;
template class DsaEngine<RingSubmitter, dsa::TasSpinlockTaskQueue>;
template class DsaEngine<RingSubmitter, dsa::SpinlockTaskQueue>;
template class DsaEngine<RingSubmitter, dsa::BackoffSpinlockTaskQueue>;
template class DsaEngine<RingSubmitter, dsa::LockFreeTaskQueue>;
