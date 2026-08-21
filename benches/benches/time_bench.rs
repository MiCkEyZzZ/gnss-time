use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use gnss_time::{Duration, DurationParts, Gps, Time};

fn bench_u64_add(c: &mut Criterion) {
    c.bench_function("u64 add", |b| {
        b.iter(|| {
            let a = black_box(1u64);
            let b = black_box(2u64);
            black_box(a + b)
        })
    });
}

fn bench_time_add_duration(c: &mut Criterion) {
    c.bench_function("Time<Gps> + Duration", |b| {
        b.iter(|| {
            let t = black_box(Time::<Gps>::from_seconds(1_000_000));
            let d = black_box(Duration::from_seconds(10));
            black_box(t + d)
        })
    });
}

fn bench_time_sub_duration(c: &mut Criterion) {
    c.bench_function("Time<Gps> - Duration", |b| {
        b.iter(|| {
            let t = black_box(Time::<Gps>::from_seconds(1_000_000));
            let d = black_box(Duration::from_seconds(10));
            black_box(t - d)
        })
    });
}

fn bench_time_diff(c: &mut Criterion) {
    c.bench_function("Time<Gps> diff", |b| {
        b.iter(|| {
            let a = black_box(Time::<Gps>::from_seconds(1_000_000));
            let b = black_box(Time::<Gps>::from_seconds(999_000));
            black_box(a - b)
        })
    });
}

fn bench_from_nanos(c: &mut Criterion) {
    c.bench_function("Time::from_nanos", |b| {
        b.iter(|| black_box(Time::<Gps>::from_nanos(1_000_000_000)))
    });
}

fn bench_from_week_tow(c: &mut Criterion) {
    c.bench_function("Time::from_week_tow (validated)", |b| {
        b.iter(|| {
            black_box(
                Time::<Gps>::from_week_tow(
                    black_box(2345),
                    black_box(DurationParts {
                        seconds: 432_000,
                        nanos: 0,
                    }),
                )
                .unwrap(),
            )
        })
    });
}

fn bench_to_tai(c: &mut Criterion) {
    c.bench_function("Time<Gps> -> TAI", |b| {
        b.iter(|| {
            let t = black_box(Time::<Gps>::from_seconds(1_000_000));
            black_box(t.to_tai().unwrap())
        })
    });
}

criterion_group!(
    time_benches,
    bench_u64_add,
    bench_time_add_duration,
    bench_time_sub_duration,
    bench_time_diff,
    bench_from_nanos,
    bench_from_week_tow,
    bench_to_tai
);

criterion_main!(time_benches);
