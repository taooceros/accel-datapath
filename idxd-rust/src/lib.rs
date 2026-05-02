//! Thin Rust wrappers over `idxd-sys` raw IDXD bindings.
//!
//! `idxd-sys` owns generated UAPI and raw MMIO portal primitives. This crate
//! only restores Rust-side alignment and typed accessors for the raw descriptor,
//! completion, work-queue, and minimal polling-future pieces currently needed by
//! DSA and IAX crc64. Higher-level sessions, retries, validation, runtime
//! integration, and lifecycle policy are intentionally absent.

pub mod idxd_async;
pub mod raw;

pub use idxd_async::{DsaEngine, DsaOperation};
pub use raw::dsa::{
    DsaCompletionRecord, DsaCompletionStatus, DsaDifCheck, DsaDifInsert, DsaDifUpdate, DsaFlag,
    DsaFlags, DsaHwDesc, DsaOpcode, default_completion_flags,
};
pub use raw::iax_crc64::{IaxCompletionRecord, IaxCompletionStatus, IaxHwDesc, IaxOpcode};
pub use raw::work_queue::{WqPortal, detect_wq_mode};
