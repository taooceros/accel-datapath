use criterion::{criterion_group, criterion_main, Criterion};

fn bench_tokio_spawn(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("tokio_spawn_join", |b| {
        b.iter(|| {
            rt.block_on(async {
                tokio::spawn(async {}).await.unwrap();
            })
        })
    });
}

criterion_group!(benches, bench_tokio_spawn);
criterion_main!(benches);
