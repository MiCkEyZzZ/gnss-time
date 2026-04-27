use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use gnss_time::{
    leap::LeapSeconds, Beidou, Galileo, Glonass, Gps, IntoScale, IntoScaleWith, Time, Utc,
};

fn bench_conversions(c: &mut Criterion) {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_week_tow(2200, 0.0).unwrap();

    c.bench_function("GPS → TAI", |b| {
        b.iter(|| {
            let tai: Time<gnss_time::Tai> = black_box(gps).into_scale().unwrap();
            black_box(tai)
        })
    });

    c.bench_function("GPS → Galileo (identity)", |b| {
        b.iter(|| {
            let gal: Time<Galileo> = black_box(gps).into_scale().unwrap();
            black_box(gal)
        })
    });

    c.bench_function("GPS → BeiDou (fixed offset)", |b| {
        b.iter(|| {
            let bdt: Time<Beidou> = black_box(gps).into_scale().unwrap();
            black_box(bdt)
        })
    });

    c.bench_function("GPS → UTC (with leap seconds)", |b| {
        b.iter(|| {
            let utc: Time<Utc> = black_box(gps).into_scale_with(ls).unwrap();
            black_box(utc)
        })
    });

    let glo = Time::<Glonass>::from_day_tod(10_000, 43_200.0).unwrap();

    c.bench_function("GLONASS → UTC (constant shift)", |b| {
        b.iter(|| {
            let utc: Time<Utc> = black_box(glo).into_scale().unwrap();
            black_box(utc)
        })
    });

    c.bench_function("GLONASS → GPS (via UTC + LS)", |b| {
        b.iter(|| {
            let gps: Time<Gps> = black_box(glo).into_scale_with(ls).unwrap();
            black_box(gps)
        })
    });
}

criterion_group!(benches, bench_conversions);
criterion_main!(benches);
