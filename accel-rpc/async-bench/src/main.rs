//! Rust async framework overhead characterization.
//!
//! The `async_overhead` Criterion suite exposes a stable control-floor set for
//! comparing executor/control-path overhead against later payload work:
//! - `tokio_spawn_join`
//! - `tokio_oneshot_completion`
//! - `tokio_mpsc_round_trip`
//! - `tokio_same_thread_wake`
//! - `tokio_cross_thread_wake`
//!
//! Export summary artifacts with:
//! `python3 async-bench/scripts/export_control_floor.py --criterion-root target/criterion --out <path>`

fn main() {
    println!(
        "async-bench: run `cargo bench -p async-bench --bench async_overhead`; export the stable summary with async-bench/scripts/export_control_floor.py"
    );
}
