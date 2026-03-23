use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use hw_eval::sw::*;

fn bench_sw_memcpy(c: &mut Criterion) {
    let mut group = c.benchmark_group("sw_memcpy");
    for size in [64, 256, 1024, 4096, 16384, 65536] {
        group.throughput(Throughput::Bytes(size as u64));
        let src = vec![0xABu8; size];
        let mut dst = vec![0u8; size];
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                sw_memcpy(&mut dst, &src);
                std::hint::black_box(&dst);
            })
        });
    }
    group.finish();
}

fn bench_sw_crc32c(c: &mut Criterion) {
    let mut group = c.benchmark_group("sw_crc32c");
    for size in [64, 256, 1024, 4096, 16384, 65536] {
        group.throughput(Throughput::Bytes(size as u64));
        let data = vec![0xABu8; size];
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let crc = sw_crc32c(&data, 0);
                std::hint::black_box(crc);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_sw_memcpy, bench_sw_crc32c);
criterion_main!(benches);
