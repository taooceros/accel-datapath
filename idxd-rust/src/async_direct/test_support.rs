use std::collections::{HashMap, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use bytes::buf::UninitSlice;
use idxd_sys::{DsaHwDesc, EnqcmdSubmission};

use crate::CompletionSnapshot;

use super::DirectMemmoveBackend;

#[derive(Debug, Clone)]
pub struct ScriptedDirectBackend {
    inner: Arc<ScriptedInner>,
}

#[derive(Debug, Default)]
struct ScriptedInner {
    submissions: AtomicUsize,
    completions: AtomicUsize,
    scripts: Mutex<VecDeque<EnqcmdSubmission>>,
    snapshots: Mutex<HashMap<u64, CompletionSnapshot>>,
    copy_on_success: bool,
}

impl ScriptedDirectBackend {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ScriptedInner {
                copy_on_success: true,
                ..ScriptedInner::default()
            }),
        }
    }

    pub fn with_submissions(submissions: impl IntoIterator<Item = EnqcmdSubmission>) -> Self {
        let backend = Self::new();
        *backend.inner.scripts.lock().expect("script lock poisoned") =
            submissions.into_iter().collect();
        backend
    }

    pub fn submissions(&self) -> usize {
        self.inner.submissions.load(Ordering::SeqCst)
    }

    pub fn completions(&self) -> usize {
        self.inner.completions.load(Ordering::SeqCst)
    }

    pub fn complete(&self, op_id: u64, snapshot: CompletionSnapshot) {
        self.inner
            .snapshots
            .lock()
            .expect("snapshot lock poisoned")
            .insert(op_id, snapshot);
    }

    pub fn clear_completion(&self, op_id: u64) {
        self.inner
            .snapshots
            .lock()
            .expect("snapshot lock poisoned")
            .remove(&op_id);
    }

    pub fn zero_success_copy() -> Self {
        Self {
            inner: Arc::new(ScriptedInner {
                copy_on_success: false,
                ..ScriptedInner::default()
            }),
        }
    }
}

impl Default for ScriptedDirectBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectMemmoveBackend for ScriptedDirectBackend {
    fn submit(&self, _op_id: u64, _descriptor: &DsaHwDesc) -> EnqcmdSubmission {
        self.inner.submissions.fetch_add(1, Ordering::SeqCst);
        self.inner
            .scripts
            .lock()
            .expect("script lock poisoned")
            .pop_front()
            .unwrap_or(EnqcmdSubmission::Accepted)
    }

    fn completion_snapshot(
        &self,
        op_id: u64,
        _state: &crate::direct_memmove::DirectMemmoveState,
    ) -> Option<CompletionSnapshot> {
        self.inner
            .snapshots
            .lock()
            .expect("snapshot lock poisoned")
            .remove(&op_id)
    }

    fn initialize_success_destination(&self, _op_id: u64, dst: &mut UninitSlice, src: &[u8]) {
        self.inner.completions.fetch_add(1, Ordering::SeqCst);
        if self.inner.copy_on_success {
            dst.copy_from_slice(src);
        } else {
            let zeros = vec![0; src.len()];
            dst.copy_from_slice(&zeros);
        }
    }
}
