//! Raw Intel IDXD bindgen/UAPI and MMIO portal primitives.
//!
//! This crate deliberately stops at the raw boundary: generated kernel UAPI
//! definitions plus the unsafe 64-byte descriptor submission doorbells. Typed
//! descriptors, completion interpretation, retry policy, CRC helpers, cache/
//! timing utilities, and safe lifecycle wrappers live in `idxd-rust`.

/// Bindgen-backed subset of the kernel `linux/idxd.h` UAPI used by IDXD
/// accelerator consumers, including DSA descriptor/completion ABI and IAX
/// definitions.
pub mod idxd_uapi {
    #![allow(
        non_camel_case_types,
        non_upper_case_globals,
        non_snake_case,
        dead_code
    )]
    include!(concat!(env!("OUT_DIR"), "/idxd_uapi_bindings.rs"));
}

/// Backward-compatible alias for existing callers that imported the generated
/// IDXD UAPI subset as `idxd_sys::idxd`.
pub use idxd_uapi as idxd;

mod portal;

pub use portal::WqPortal;
