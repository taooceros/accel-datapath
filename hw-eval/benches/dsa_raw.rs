use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use hw_eval::dsa::*;
use std::path::Path;

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

fn bench_dsa_memmove(c: &mut Criterion) {
    let wq = match WqPortal::open(Path::new("/dev/dsa/wq0.0")) {
        Ok(wq) => wq,
        Err(_) => {
            eprintln!("Skipping DSA benchmarks — cannot open /dev/dsa/wq0.0");
            return;
        }
    };

    let mut group = c.benchmark_group("dsa_memmove");
    for size in [64, 256, 1024, 4096, 16384, 65536] {
        group.throughput(Throughput::Bytes(size as u64));
        let src = vec![0xABu8; size];
        let mut dst = vec![0u8; size];
        let mut desc = DsaHwDesc::default();
        let mut comp = DsaCompletionRecord::default();

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                reset_completion(&mut comp);
                desc.fill_memmove(src.as_ptr(), dst.as_mut_ptr(), size as u32);
                desc.set_completion(&mut comp);
                unsafe { wq.submit(&desc) };
                poll_completion(&comp);
            })
        });
    }
    group.finish();
}

fn bench_dsa_copy_crc(c: &mut Criterion) {
    let wq = match WqPortal::open(Path::new("/dev/dsa/wq0.0")) {
        Ok(wq) => wq,
        Err(_) => {
            eprintln!("Skipping DSA benchmarks — cannot open /dev/dsa/wq0.0");
            return;
        }
    };

    let mut group = c.benchmark_group("dsa_copy_crc");
    for size in [64, 256, 1024, 4096, 16384, 65536] {
        group.throughput(Throughput::Bytes(size as u64));
        let src = vec![0xABu8; size];
        let mut dst = vec![0u8; size];
        let mut desc = DsaHwDesc::default();
        let mut comp = DsaCompletionRecord::default();

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                reset_completion(&mut comp);
                desc.fill_copy_crc(src.as_ptr(), dst.as_mut_ptr(), size as u32, 0);
                desc.set_completion(&mut comp);
                unsafe { wq.submit(&desc) };
                let _ = poll_completion(&comp);
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_sw_memcpy,
    bench_sw_crc32c,
    bench_dsa_memmove,
    bench_dsa_copy_crc,
);
criterion_main!(benches);
