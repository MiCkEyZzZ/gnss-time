use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use gnss_time::{Duration, Gps, Time};

fn bench_arithmetic(c: &mut Criterion) {
    let time = Time::<Gps>::from_seconds(1_000_000_000);
    let dur = Duration::from_seconds(123_456);

    let raw_time = 1_000_000_000u64;
    let raw_dur = 123_456u64;

    let mut group = c.benchmark_group("arithmetic");

    group.bench_function("Time<Gps> + Duration", |b| b.iter(|| black_box(time + dur)));

    group.bench_function("u64 + u64 (raw)", |b| {
        b.iter(|| black_box(raw_time + raw_dur))
    });

    group.bench_function("Time<Gps> - Duration", |b| b.iter(|| black_box(time - dur)));

    group.bench_function("u64 - u64 (raw)", |b| {
        b.iter(|| black_box(raw_time - raw_dur))
    });

    group.finish();
}

fn bench_saturating(c: &mut Criterion) {
    let time = Time::<Gps>::MAX;
    let dur = Duration::from_seconds(1);

    c.bench_function("Time<Gps>::saturating_add", |b| {
        b.iter(|| black_box(time.saturating_add(dur)))
    });
}

criterion_group!(benches, bench_arithmetic, bench_saturating);
criterion_main!(benches);
