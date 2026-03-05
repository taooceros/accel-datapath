//! Accelerator-aware Codec for Tonic gRPC.
//!
//! Replaces ProstCodec with a custom codec that uses pooled, pre-faulted buffers
//! suitable for hardware accelerator (DSA/IAX) operations.
