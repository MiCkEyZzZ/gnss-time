//! Head-to-head: `gnss-time` vs `hifitime` (4.x).
//!
//! Both crates model GNSS/atomic time with leap-second awareness. This
//! benchmark measures semantically equivalent operations:
//!
//! | Operation              | gnss-time                        | hifitime                    |
//! | ---------------------- | -------------------------------- | --------------------------- |
//! | construct GPS          | `Time::<Gps>::from_week_tow`     | `Epoch::from_gpst_seconds`  |
//! | add duration           | `Time<Gps> + Duration`           | `Epoch + Duration`          |
//! | fixed scale conversion | `into_scale::<Tai>()`            | `to_tai_duration()`         |
//! | leap-aware conversion  | `gps_to_utc` (builtin table)     | `to_utc_duration()`         |
//!
//! Fairness notes:
//! - hifitime's `Epoch` is 24 bytes (`i16 centuries + u64 ns + TimeScale`);
//!   `gnss-time`'s `Time<S>` is 8 bytes. Memory footprint is a structural
//!   difference, not a microbenchmark target here.
//! - hifitime maintains the time scale inside the type and normalizes the
//!   century counter on most operations, which is part of its richer model.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use gnss_time::{gps_to_utc, DurationParts, Gps, IntoScale, LeapSeconds, Tai, Time};
use hifitime::Epoch;

// GPS week 2345, TOW 432000  ≈ 2025-01-01 (1_418_688_000 s past GPS epoch).
const GPS_SECONDS: f64 = 1_418_688_000.0;

// -- Construction -----------------------------------------------------------

fn bench_construct_gps(c: &mut Criterion) {
    c.bench_function("construct GPS — gnss-time from_week_tow", |b| {
        b.iter(|| {
            black_box(
                Time::<Gps>::from_week_tow(
                    2345,
                    DurationParts {
                        seconds: 432_000,
                        nanos: 0,
                    },
                )
                .unwrap(),
            )
        })
    });

    c.bench_function("construct GPS — hifitime from_gpst_seconds", |b| {
        b.iter(|| black_box(Epoch::from_gpst_seconds(GPS_SECONDS)))
    });
}

// -- Addition ---------------------------------------------------------------

fn bench_add_duration(c: &mut Criterion) {
    let t = black_box(Time::<Gps>::from_seconds(1_000_000));
    let d = black_box(gnss_time::Duration::from_seconds(1));
    let e = black_box(Epoch::from_gpst_seconds(GPS_SECONDS));
    let hd = black_box(hifitime::Duration::from_seconds(1.0));

    c.bench_function("add duration — gnss-time Time + Duration", |b| {
        b.iter(|| black_box(t + d))
    });

    c.bench_function("add duration — hifitime Epoch + Duration", |b| {
        b.iter(|| black_box(e + hd))
    });
}

// -- Fixed scale conversion (GPS → TAI) --------------------------------------

fn bench_gps_to_tai(c: &mut Criterion) {
    let t = black_box(
        Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap(),
    );
    let e = black_box(Epoch::from_gpst_seconds(GPS_SECONDS));

    c.bench_function("GPS → TAI — gnss-time into_scale::<Tai>", |b| {
        b.iter(|| {
            let tai: Time<Tai> = black_box(t).into_scale().unwrap();
            black_box(tai)
        })
    });

    c.bench_function("GPS → TAI — hifitime to_tai_duration", |b| {
        b.iter(|| black_box(black_box(e).to_tai_duration()))
    });
}

// -- Leap-second-aware conversion (GPS → UTC) --------------------------------

fn bench_gps_to_utc(c: &mut Criterion) {
    let t = black_box(
        Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap(),
    );
    let ls = LeapSeconds::builtin();
    let e = black_box(Epoch::from_gpst_seconds(GPS_SECONDS));

    c.bench_function("GPS → UTC — gnss-time gps_to_utc", |b| {
        b.iter(|| black_box(gps_to_utc(black_box(t), ls).unwrap()))
    });

    c.bench_function("GPS → UTC — hifitime to_utc_duration", |b| {
        b.iter(|| black_box(black_box(e).to_utc_duration()))
    });
}

// -- Type sizes --------------------------------------------------------------

fn assert_type_sizes(_c: &mut Criterion) {
    use core::mem::size_of;
    assert_eq!(
        size_of::<Time<Gps>>(),
        8,
        "gnss-time Time<S> must be 8 bytes"
    );
    assert_eq!(
        size_of::<Epoch>(),
        24,
        "hifitime Epoch is structurally larger (i16 centuries + u64 ns + TimeScale)"
    );
}

criterion_group!(
    vs_hifitime,
    assert_type_sizes,
    bench_construct_gps,
    bench_add_duration,
    bench_gps_to_tai,
    bench_gps_to_utc,
);
criterion_main!(vs_hifitime);
