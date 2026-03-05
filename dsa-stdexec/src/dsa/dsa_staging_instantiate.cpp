#include "dsa.hpp"

// Double-buffered batch (was DsaBatchBase)
template class DsaEngine<StagingSubmitter, dsa::MutexTaskQueue>;
template class DsaEngine<StagingSubmitter, dsa::SingleThreadTaskQueue>;
template class DsaEngine<StagingSubmitter, dsa::TasSpinlockTaskQueue>;
template class DsaEngine<StagingSubmitter, dsa::SpinlockTaskQueue>;
template class DsaEngine<StagingSubmitter, dsa::BackoffSpinlockTaskQueue>;
template class DsaEngine<StagingSubmitter, dsa::LockFreeTaskQueue>;
