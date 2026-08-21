//! Leap-second context benchmarks.
//!
//! Compares `GPS → UTC` and `UTC → GPS` across different
//! [`LeapSecondsProvider`] implementations and contrasts the public
//! two-pass `utc_to_gps` algorithm with a single-pass reference
//! implementation (one table lookup instead of two).
//!
//! Reference numbers and tables live in `benches/README.md`.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use gnss_time::{
    gps_to_utc, utc_to_gps, DurationParts, Gps, LeapSeconds, LeapSecondsProvider,
    RuntimeLeapSeconds, Tai, Time,
};

/// GPS week 2086, TOW 0 → 2020-01-06 00:00:00 UTC (TAI−UTC = 37).
const WEEK: u16 = 2086;

/// TAI nanoseconds for the same instant (2020-01-06 + 37 s).
const TAI_2020_NS: u64 = 1_262_304_037_000_000_000;

/// Offset between the crate's UTC epoch (1972-01-01) and the GPS epoch
/// (1980-01-06): 2927 days in nanoseconds. Mirrors the private
/// `UTC_TO_GPS_EPOCH_NS` constant of the crate.
const UTC_TO_GPS_EPOCH_NS: i128 = 252_892_800_000_000_000;

/// Receiver-style provider: a single constant offset taken from the
/// navigation message instead of a full table.
struct FixedOffset(i32);

impl LeapSecondsProvider for FixedOffset {
    fn tai_minus_utc_at(
        &self,
        _tai: Time<Tai>,
    ) -> i32 {
        self.0
    }
}

fn bench_gps_to_utc_providers(c: &mut Criterion) {
    let gps = black_box(
        Time::<Gps>::from_week_tow(
            WEEK,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap(),
    );

    let builtin = LeapSeconds::builtin();
    let runtime_full = RuntimeLeapSeconds::from_builtin();
    let runtime_empty = RuntimeLeapSeconds::from_slice(&[]).unwrap();
    let custom = FixedOffset(37);

    let mut group = c.benchmark_group("GPS → UTC / leap second provider");

    group.bench_function("builtin static table", |b| {
        b.iter(|| black_box(gps_to_utc(black_box(gps), builtin).unwrap()))
    });

    group.bench_function("RuntimeLeapSeconds (19 entries)", |b| {
        b.iter(|| black_box(gps_to_utc(black_box(gps), &runtime_full).unwrap()))
    });

    group.bench_function("RuntimeLeapSeconds (empty, fallback)", |b| {
        b.iter(|| black_box(gps_to_utc(black_box(gps), &runtime_empty).unwrap()))
    });

    group.bench_function("custom constant (receiver-style)", |b| {
        b.iter(|| black_box(gps_to_utc(black_box(gps), &custom).unwrap()))
    });

    group.finish();
}

fn bench_tai_minus_utc_providers(c: &mut Criterion) {
    let tai = black_box(Time::<Tai>::from_nanos(TAI_2020_NS));

    let builtin = LeapSeconds::builtin();
    let runtime_full = RuntimeLeapSeconds::from_builtin();
    let runtime_empty = RuntimeLeapSeconds::from_slice(&[]).unwrap();
    let custom = FixedOffset(37);

    let mut group = c.benchmark_group("tai_minus_utc_at / leap second provider");

    group.bench_function("builtin static table", |b| {
        b.iter(|| black_box(builtin.tai_minus_utc_at(black_box(tai))))
    });

    group.bench_function("RuntimeLeapSeconds (19 entries)", |b| {
        b.iter(|| black_box(runtime_full.tai_minus_utc_at(black_box(tai))))
    });

    group.bench_function("RuntimeLeapSeconds (empty, fallback)", |b| {
        b.iter(|| black_box(runtime_empty.tai_minus_utc_at(black_box(tai))))
    });

    group.bench_function("custom constant (receiver-style)", |b| {
        b.iter(|| black_box(custom.tai_minus_utc_at(black_box(tai))))
    });

    group.finish();
}

fn bench_utc_to_gps_passes(c: &mut Criterion) {
    let gps = Time::<Gps>::from_week_tow(
        WEEK,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let ls = LeapSeconds::builtin();
    let utc = black_box(gps_to_utc(gps, ls).unwrap());

    // Pre-computed first-pass approximation (shared by both variants),
    // so the loop body isolates the actual difference: one vs two
    // binary searches.
    let approx_tai_ns = i128::from(utc.as_nanos()) - UTC_TO_GPS_EPOCH_NS + 19_000_000_000_i128;
    let approx_tai = black_box(Time::<Tai>::from_nanos(approx_tai_ns as u64));
    let utc_ns = black_box(i128::from(utc.as_nanos()));

    let mut group = c.benchmark_group("UTC → GPS / algorithm");

    group.bench_function("two-pass (public API)", |b| {
        b.iter(|| black_box(utc_to_gps(black_box(utc), ls).unwrap()))
    });

    // Single-pass reference: one table lookup + arithmetic. Not part of
    // the public API — a baseline showing what a single-binary-search
    // implementation would cost.
    group.bench_function("one-pass reference (single lookup)", |b| {
        b.iter(|| {
            let n = black_box(ls.tai_minus_utc_at(black_box(approx_tai)));
            let gps_ns = utc_ns + i128::from(n - 19) * 1_000_000_000_i128 - UTC_TO_GPS_EPOCH_NS;
            black_box(Time::<Gps>::from_nanos(gps_ns as u64))
        })
    });

    group.finish();
}

criterion_group!(
    leap_context,
    bench_gps_to_utc_providers,
    bench_tai_minus_utc_providers,
    bench_utc_to_gps_passes,
);
criterion_main!(leap_context);
