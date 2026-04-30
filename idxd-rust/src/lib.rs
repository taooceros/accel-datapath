//! Thin Intel DSA memmove bridge for `tonic-profile`.
//!
//! This crate deliberately stays narrow: it opens one IDXD work queue, submits
//! one memmove descriptor at a time through `idxd-sys`, retries recoverable
//! page faults, verifies copied bytes, and maps queue-open/completion failures
//! into typed Rust errors.
//!
//! Synchronous callers pass explicit source and destination slices to
//! `DsaSession::memmove`; request validation always treats the source length as
//! the requested transfer size and only requires destination capacity to be at
//! least that large. Async callers should use `AsyncMemmoveRequest::new` when
//! work must cross Tokio tasks: requests own a `bytes::Bytes` source and a
//! caller-provided `bytes::BytesMut` destination, and `AsyncMemmoveResult`
//! returns the owned destination plus validation report after direct completion
//! record observation. The async API intentionally has no public allocation
//! convenience constructor and no borrowed copy-back helper; destination
//! allocation and ownership stay explicit at the call site.
//!
//! `IdxdSession<Accel>` is the generic IDXD architecture direction for the sealed `Dsa`
//! and `Iax`/`Iaa` marker families. It opens one work queue and now carries narrow
//! representative operations: `IdxdSession<Dsa>::memmove` reuses the same blocking DSA
//! lifecycle as `DsaSession`, while `IdxdSession<Iax>::crc64`/`IdxdSession<Iaa>::crc64`
//! use an IAX-owned descriptor/completion interpreter. This is intentionally not full
//! DSA/IAX coverage and does not introduce a public operation trait hierarchy or runtime
//! accelerator dispatch.
//!
//! Compatibility DSA session code lives in `legacy_dsa`; hidden worker-fixture code lives
//! under `async_session::legacy_worker`. Keeping those files separate makes the current
//! generic-session path easier to read without breaking existing public imports.

mod async_direct;
mod async_session;
mod direct_memmove;
mod iax_crc64;
mod legacy_dsa;
mod lifecycle;
mod session;
mod validation;

#[doc(hidden)]
pub use async_direct::test_support as direct_test_support;
pub use async_direct::{
    AsyncDirectFailure, AsyncDirectFailureKind, DirectAsyncMemmoveRuntime, DirectMemmoveBackend,
    DirectPortalBackend,
};
pub use async_session::{
    AsyncDsaHandle, AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveError,
    AsyncMemmoveRequest, AsyncMemmoveRequestError, AsyncMemmoveResult, AsyncMemmoveWorker,
    AsyncWorkerFailureKind,
};
pub use iax_crc64::{
    IAX_CRC64_COMPLETION_TIMEOUT_STATUS, IaxCrc64Error, IaxCrc64Phase, IaxCrc64Report,
    IaxCrc64Result, MAX_IAX_CRC64_BYTES,
};
pub use legacy_dsa::DsaSession;
pub use session::{Accelerator, Dsa, Iaa, Iax, IdxdSession, IdxdSessionConfig, IdxdSessionError};
pub use validation::{
    COMPLETION_TIMEOUT_STATUS, CompletionAction, CompletionSnapshot, DEFAULT_DEVICE_PATH,
    DEFAULT_MAX_PAGE_FAULT_RETRIES, DsaConfig, MAX_MEMMOVE_BYTES, MemmoveError, MemmovePhase,
    MemmoveRequest, MemmoveRetry, MemmoveValidationReport, classify_memmove_completion,
};
