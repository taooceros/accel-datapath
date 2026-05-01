use std::sync::{Arc, Weak, atomic::Ordering};

use super::{DirectMemmoveBackend, MONITOR_IDLE_BACKOFF, RuntimeInner};

pub(super) async fn monitor_completion_records<B>(inner: Weak<RuntimeInner<B>>)
where
    B: DirectMemmoveBackend,
{
    loop {
        let Some(inner) = inner.upgrade() else {
            return;
        };

        let operations = pending_operations(&inner);

        if operations.is_empty() {
            if inner.closed.load(Ordering::Acquire) {
                return;
            }
            let pending_notify = Arc::clone(&inner.pending_notify);
            drop(inner);
            tokio::select! {
                _ = pending_notify.notified() => {}
                _ = tokio::time::sleep(MONITOR_IDLE_BACKOFF) => {}
            }
            continue;
        }

        for operation in operations {
            if let Some(snapshot) = operation.completion_snapshot(&inner.backend) {
                let terminal = operation.handle_snapshot(&inner, snapshot).await;
                if terminal {
                    inner
                        .pending
                        .lock()
                        .expect("pending registry poisoned")
                        .remove(&operation.id());
                }
            }
        }

        drop(inner);
        tokio::task::yield_now().await;
    }
}

fn pending_operations<B>(inner: &RuntimeInner<B>) -> Vec<Arc<super::operation::PendingOperation>> {
    let pending = inner.pending.lock().expect("pending registry poisoned");
    pending.values().cloned().collect()
}
