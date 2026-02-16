#include "dsa.hpp"

// Mirrored ring batch (wrap-free via virtual memory mirroring)
template class DsaEngine<MirroredRingSubmitter, dsa::MutexTaskQueue>;
template class DsaEngine<MirroredRingSubmitter, dsa::SingleThreadTaskQueue>;
template class DsaEngine<MirroredRingSubmitter, dsa::TasSpinlockTaskQueue>;
template class DsaEngine<MirroredRingSubmitter, dsa::SpinlockTaskQueue>;
template class DsaEngine<MirroredRingSubmitter, dsa::BackoffSpinlockTaskQueue>;
template class DsaEngine<MirroredRingSubmitter, dsa::LockFreeTaskQueue>;
