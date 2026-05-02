use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use gnss_time::{
    gps_to_utc, utc_to_gps, Beidou, DurationParts, Galileo, Gps, IntoScale, LeapSeconds, Tai, Time,
};

fn bench_gps_to_tai(c: &mut Criterion) {
    let gps = black_box(
        Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap(),
    );

    c.bench_function("GPS → TAI (fixed +19s)", |b| {
        b.iter(|| {
            let tai: Time<Tai> = black_box(gps).into_scale().unwrap();
            black_box(tai)
        })
    });
}

fn bench_gps_to_galileo(c: &mut Criterion) {
    let gps = black_box(
        Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap(),
    );

    c.bench_function("GPS → Galileo (identity)", |b| {
        b.iter(|| {
            let gal: Time<Galileo> = black_box(gps).into_scale().unwrap();
            black_box(gal)
        })
    });
}

fn bench_gps_to_beidou(c: &mut Criterion) {
    let gps = black_box(
        Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap(),
    );

    c.bench_function("GPS → BeiDou (fixed -14s via TAI)", |b| {
        b.iter(|| {
            let bdt: Time<Beidou> = black_box(gps).into_scale().unwrap();
            black_box(bdt)
        })
    });
}

fn bench_tai_to_gps(c: &mut Criterion) {
    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000,
            nanos: 0,
        },
    )
    .unwrap();
    let tai: Time<Tai> = gps.into_scale().unwrap();
    let tai = black_box(tai);

    c.bench_function("TAI → GPS (fixed -19s)", |b| {
        b.iter(|| {
            let g: Time<Gps> = black_box(tai).into_scale().unwrap();
            black_box(g)
        })
    });
}

fn bench_gps_to_utc(c: &mut Criterion) {
    let gps = black_box(
        Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap(),
    );
    let ls = LeapSeconds::builtin();

    c.bench_function("GPS → UTC (builtin table, 2020)", |b| {
        b.iter(|| black_box(gps_to_utc(black_box(gps), ls).unwrap()))
    });
}

fn bench_gps_to_utc_at_epoch(c: &mut Criterion) {
    let gps = black_box(Time::<Gps>::EPOCH);
    let ls = LeapSeconds::builtin();

    c.bench_function("GPS → UTC (builtin table, GPS epoch 1980)", |b| {
        b.iter(|| black_box(gps_to_utc(black_box(gps), ls).unwrap()))
    });
}

fn bench_utc_to_gps(c: &mut Criterion) {
    // Pre-compute a UTC value near 2020
    let gps = Time::<Gps>::from_week_tow(
        2086,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let ls = LeapSeconds::builtin();
    let utc = black_box(gps_to_utc(gps, ls).unwrap());

    c.bench_function("UTC → GPS (two-pass algorithm, 2020)", |b| {
        b.iter(|| black_box(utc_to_gps(black_box(utc), ls).unwrap()))
    });
}

fn bench_gps_utc_roundtrip(c: &mut Criterion) {
    let gps = black_box(
        Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap(),
    );
    let ls = LeapSeconds::builtin();

    c.bench_function("GPS → UTC → GPS (full roundtrip)", |b| {
        b.iter(|| {
            let utc = gps_to_utc(black_box(gps), ls).unwrap();
            let back = utc_to_gps(utc, ls).unwrap();
            black_box(back)
        })
    });
}

fn bench_leap_second_lookup(c: &mut Criterion) {
    use gnss_time::{leap::LeapSecondsProvider, scale::Tai, Time};

    let ls = LeapSeconds::builtin();
    // TAI in 2020
    let tai = black_box(Time::<Tai>::from_nanos(1_262_304_037_000_000_000));

    c.bench_function("LeapSeconds::builtin binary_search (19 entries)", |b| {
        b.iter(|| black_box(ls.tai_minus_utc_at(black_box(tai))))
    });
}

criterion_group!(
    conversions,
    bench_gps_to_tai,
    bench_gps_to_galileo,
    bench_gps_to_beidou,
    bench_tai_to_gps,
    bench_gps_to_utc,
    bench_gps_to_utc_at_epoch,
    bench_utc_to_gps,
    bench_gps_utc_roundtrip,
    bench_leap_second_lookup,
);
criterion_main!(conversions);
