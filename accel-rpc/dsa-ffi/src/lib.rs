//! FFI bridge to Intel DSA hardware via the C++ dsa-stdexec codebase.
//!
//! Exposes DSA operations (copy_crc, data_move, crc_gen) as async Rust Futures
//! that poll hardware completion records.
