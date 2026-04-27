use async_bench::bench_support::{
    mpsc_round_trip, oneshot_completion_round_trip, same_thread_wake_round_trip,
    spawn_join_round_trip, CrossThreadWakeHarness,
};
use criterion::{criterion_group, criterion_main, Criterion};

const SUITE_NAME: &str = "async_control_floor";

fn bench_async_control_floor(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build benchmark runtime");
    let cross_thread_wake = CrossThreadWakeHarness::new();
    let mut group = c.benchmark_group(SUITE_NAME);

    group.bench_function("tokio_spawn_join", |b| {
        b.to_async(&runtime).iter(|| async {
            spawn_join_round_trip()
                .await
                .expect("tokio_spawn_join must complete a real spawn/join round trip");
        })
    });

    group.bench_function("tokio_oneshot_completion", |b| {
        b.to_async(&runtime).iter(|| async {
            oneshot_completion_round_trip()
                .await
                .expect("tokio_oneshot_completion must complete both send and receive");
        })
    });

    group.bench_function("tokio_mpsc_round_trip", |b| {
        b.to_async(&runtime).iter(|| async {
            mpsc_round_trip()
                .await
                .expect("tokio_mpsc_round_trip must complete both send and receive");
        })
    });

    group.bench_function("tokio_same_thread_wake", |b| {
        b.to_async(&runtime).iter(|| async {
            same_thread_wake_round_trip()
                .await
                .expect("tokio_same_thread_wake must complete a same-thread wake cycle");
        })
    });

    group.bench_function("tokio_cross_thread_wake", |b| {
        b.to_async(&runtime).iter(|| async {
            cross_thread_wake
                .round_trip()
                .await
                .expect("tokio_cross_thread_wake must complete a cross-thread wake cycle");
        })
    });

    group.finish();
}

criterion_group!(benches, bench_async_control_floor);
criterion_main!(benches);
