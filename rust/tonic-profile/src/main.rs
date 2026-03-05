//! Tonic profiling harness.
//!
//! Baseline gRPC server + client for profiling the Tonic data path.
//! Used to identify CPU hotspots: serialization, compression, buffer copies,
//! HTTP/2 framing, and network I/O.

fn main() {
    println!("tonic-profile: profiling harness (TODO)");
}
