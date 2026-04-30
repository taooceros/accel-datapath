//! Low-level Intel DSA hardware bindings — zero framework overhead.
//!
//! Directly maps the WQ portal, fills descriptors, submits via MOVDIR64B/ENQCMD,
//! and polls completion records. No allocators, no async, no abstractions.

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

mod cache;
mod completion;
mod descriptor;
mod iax;
mod portal;
mod timing;
mod topology;

pub use cache::flush_range;
pub use completion::{drain_completions, poll_completion, reset_completion, touch_fault_page};
pub use descriptor::{
    BindgenDsaCompletionRecord, BindgenDsaHwDesc, DSA_COMP_NONE, DSA_COMP_PAGE_FAULT_NOBOF,
    DSA_COMP_STATUS_MASK, DSA_COMP_SUCCESS, DSA_OPCODE_BATCH, DSA_OPCODE_CFLUSH,
    DSA_OPCODE_COMPARE, DSA_OPCODE_COMPVAL, DSA_OPCODE_COPY_CRC, DSA_OPCODE_CRCGEN,
    DSA_OPCODE_DUALCAST, DSA_OPCODE_MEMFILL, DSA_OPCODE_MEMMOVE, DSA_OPCODE_NOOP,
    DsaCompletionRecord, DsaHwDesc, IDXD_OP_FLAG_CC, IDXD_OP_FLAG_CRAV, IDXD_OP_FLAG_RCR,
};
pub use iax::{
    BindgenIaxCompletionRecord, BindgenIaxHwDesc, IAX_COMP_NONE, IAX_COMP_OUTBUF_OVERFLOW,
    IAX_COMP_PAGE_FAULT_IR, IAX_COMP_STATUS_MASK, IAX_COMP_SUCCESS, IAX_CRC64_FLAGS_OFFSET,
    IAX_CRC64_POLY_OFFSET, IAX_CRC64_POLY_T10DIF, IAX_CRC64_RESULT_OFFSET, IAX_OPCODE_COMPRESS,
    IAX_OPCODE_CRC64, IAX_OPCODE_DECOMPRESS, IAX_OPCODE_MEMMOVE, IAX_OPCODE_NOOP,
    IAX_STATUS_ANALYTICS_ERROR, IaxCompletionRecord, IaxHwDesc, crc16_t10dif, crc64_t10dif_field,
    drain_iax_completions, poll_iax_completion, reset_iax_completion, touch_iax_fault_page,
};
pub use portal::{EnqcmdSubmission, WqPortal};
pub use timing::{cycles_to_ns, lfence, rdtscp, tsc_frequency_hz};
pub use topology::{cpu_numa_node, current_core, device_numa_node, pin_to_core};
