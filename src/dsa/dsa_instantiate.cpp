#include "dsa.hpp"

// Direct submission (was DsaBase)
template class DsaEngine<DirectSubmitter, dsa::MutexTaskQueue>;
template class DsaEngine<DirectSubmitter, dsa::SingleThreadTaskQueue>;
template class DsaEngine<DirectSubmitter, dsa::TasSpinlockTaskQueue>;
template class DsaEngine<DirectSubmitter, dsa::SpinlockTaskQueue>;
template class DsaEngine<DirectSubmitter, dsa::BackoffSpinlockTaskQueue>;
template class DsaEngine<DirectSubmitter, dsa::LockFreeTaskQueue>;
