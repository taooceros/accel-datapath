use idxd_sys::WqPortal;

/// Decision produced after an operation observes and classifies one completion.
pub(crate) enum BlockingOperationDecision<T> {
    Complete(T),
    Retry,
}

/// Crate-private static-dispatch contract for one blocking submit/complete loop.
///
/// Operation-owned state remains responsible for descriptor fill, submission
/// safety, completion interpretation, retry side effects, and typed success or
/// error construction. The lifecycle owns only the common reset/fill → submit →
/// observe → classify → retry/return control flow.
pub(crate) trait BlockingOperation {
    type Completion;
    type Output;
    type Error;

    fn reset_and_fill_descriptor(&mut self);

    /// Submit the operation-owned descriptor through the supplied portal.
    ///
    /// # Safety
    /// Implementations must submit only descriptors whose completion records and
    /// referenced buffers remain valid until hardware reaches terminal
    /// completion. The lifecycle calls this immediately after
    /// `reset_and_fill_descriptor` and keeps the operation state alive until the
    /// completion has been observed and classified.
    unsafe fn submit(&self, portal: &WqPortal);

    fn observe_completion(&self) -> Self::Completion;

    fn classify_completion(
        &mut self,
        completion: Self::Completion,
    ) -> Result<BlockingOperationDecision<Self::Output>, Self::Error>;
}

/// Run one operation state through the shared blocking submit/complete lifecycle.
pub(crate) fn run_blocking_operation<O>(
    portal: &WqPortal,
    operation: &mut O,
) -> Result<O::Output, O::Error>
where
    O: BlockingOperation,
{
    loop {
        operation.reset_and_fill_descriptor();

        // SAFETY: `BlockingOperation` implementations define the concrete
        // descriptor/completion/buffer lifetime contract and this lifecycle keeps
        // the operation value borrowed until its completion is observed and
        // classified before any retry or terminal return.
        unsafe {
            operation.submit(portal);
        }

        let completion = operation.observe_completion();
        match operation.classify_completion(completion)? {
            BlockingOperationDecision::Complete(output) => return Ok(output),
            BlockingOperationDecision::Retry => {}
        }
    }
}
