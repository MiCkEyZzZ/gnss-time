//! # Benchmark: арифметика `Time<S>` vs голый `u64`
//!
//! Цель: доказать **zero-cost abstraction** — `Time<Gps> + Duration`
//! компилируется в те же инструкции, что и `u64 + u64`.
//!
//! Ожидаемые результаты:
//! - `time_add`  ≈ `raw_u64_add`  (0 нс разницы)
//! - `time_sub`  ≈ `raw_u64_sub`  (0 нс разницы)
//! - Checked-варианты — один дополнительный branch, < 1 нс
//! - Saturating-варианты — аналогично checked

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use gnss_time::{Duration, Gps, Time};

// ── Panicking operators (должны быть идентичны u64 add/sub)
// ───────────────────

fn bench_time_add(c: &mut Criterion) {
    let t = black_box(Time::<Gps>::from_seconds(1_000_000));
    let d = black_box(Duration::from_seconds(1));

    c.bench_function("Time<Gps> + Duration (panicking)", |b| {
        b.iter(|| black_box(t + d))
    });
}

fn bench_raw_u64_add(c: &mut Criterion) {
    let t = black_box(1_000_000_u64 * 1_000_000_000);
    let d = black_box(1_000_000_000_u64);

    c.bench_function("u64 + u64 (raw baseline)", |b| b.iter(|| black_box(t + d)));
}

fn bench_time_sub(c: &mut Criterion) {
    let a = black_box(Time::<Gps>::from_seconds(2_000_000));
    let b = black_box(Time::<Gps>::from_seconds(1_000_000));

    c.bench_function("Time<Gps> - Time<Gps> (panicking)", |b_| {
        b_.iter(|| black_box(a - b))
    });
}

fn bench_raw_u64_sub(c: &mut Criterion) {
    let a = black_box(2_000_000_u64 * 1_000_000_000);
    let b = black_box(1_000_000_u64 * 1_000_000_000);

    c.bench_function("u64 - u64 (raw baseline)", |b_| {
        b_.iter(|| black_box(a - b))
    });
}

// ── Checked variants
// ──────────────────────────────────────────────────────────

fn bench_checked_add(c: &mut Criterion) {
    let t = black_box(Time::<Gps>::from_seconds(1_000_000));
    let d = black_box(Duration::from_seconds(1));

    c.bench_function("Time<Gps>.checked_add", |b| {
        b.iter(|| black_box(t.checked_add(d)))
    });
}

fn bench_checked_sub(c: &mut Criterion) {
    let t = black_box(Time::<Gps>::from_seconds(1_000_000));
    let d = black_box(Duration::from_seconds(1));

    c.bench_function("Time<Gps>.checked_sub_duration", |b| {
        b.iter(|| black_box(t.checked_sub_duration(d)))
    });
}

// ── Saturating variants
// ───────────────────────────────────────────────────────

fn bench_saturating_add(c: &mut Criterion) {
    let t = black_box(Time::<Gps>::from_seconds(1_000_000));
    let d = black_box(Duration::from_seconds(1));

    c.bench_function("Time<Gps>.saturating_add", |b| {
        b.iter(|| black_box(t.saturating_add(d)))
    });
}

fn bench_saturating_add_at_max(c: &mut Criterion) {
    let t = black_box(Time::<Gps>::MAX);
    let d = black_box(Duration::from_seconds(1));

    c.bench_function("Time<Gps>.saturating_add (at MAX, clamps)", |b| {
        b.iter(|| black_box(t.saturating_add(d)))
    });
}

// ── Duration arithmetic
// ───────────────────────────────────────────────────────

fn bench_duration_add(c: &mut Criterion) {
    let a = black_box(Duration::from_seconds(1_000));
    let b = black_box(Duration::from_nanos(500_000_000));

    c.bench_function("Duration + Duration", |b_| b_.iter(|| black_box(a + b)));
}

fn bench_duration_checked_add(c: &mut Criterion) {
    let a = black_box(Duration::from_seconds(1_000));
    let b = black_box(Duration::from_nanos(500_000_000));

    c.bench_function("Duration.checked_add", |b_| {
        b_.iter(|| black_box(a.checked_add(b)))
    });
}

criterion_group!(
    arithmetic,
    bench_time_add,
    bench_raw_u64_add,
    bench_time_sub,
    bench_raw_u64_sub,
    bench_checked_add,
    bench_checked_sub,
    bench_saturating_add,
    bench_saturating_add_at_max,
    bench_duration_add,
    bench_duration_checked_add,
);
criterion_main!(arithmetic);
