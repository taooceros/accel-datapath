//! Thin Rust wrappers over `idxd-sys` raw IDXD bindings.
//!
//! `idxd-sys` owns generated UAPI and raw MMIO portal primitives. This crate
//! only restores Rust-side alignment and typed accessors for the raw descriptor,
//! completion, and work-queue pieces currently needed by DSA memmove and IAX
//! crc64. Higher-level sessions, lifecycles, retry policy, validation, and async
//! APIs are intentionally absent.

mod raw;

pub use raw::dsa_memmove::{DsaCompletionRecord, DsaCompletionStatus, DsaHwDesc, DsaOpcode};
pub use raw::iax_crc64::{IaxCompletionRecord, IaxCompletionStatus, IaxHwDesc, IaxOpcode};
pub use raw::work_queue::WqPortal;
