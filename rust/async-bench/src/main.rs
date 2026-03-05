//! Rust async framework overhead characterization.
//!
//! Measures:
//! - tokio task spawn + join overhead
//! - Future::poll round-trip cost
//! - Channel send/recv latency
//! - spawn_blocking overhead
//! - Waker registration and wake cost
//!
//! These baselines inform how to integrate hardware accelerator completion
//! polling with the tokio async runtime.

fn main() {
    println!("async-bench: run `cargo bench` for measurements");
}
