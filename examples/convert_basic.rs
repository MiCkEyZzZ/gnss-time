//! Demonstrates fixed-offset conversions (no leap seconds needed).
//!
//! These conversions are compile-time constant and never ambiguous.

use gnss_time::prelude::*;

fn main() {
    // GPS → TAI (add 19 seconds)
    let gps = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
    let tai: Time<Tai> = gps.into_scale().unwrap();

    println!("GPS  → TAI: {} → {}", gps, tai);

    // GPS → Galileo (same instant, same nanoseconds)
    let gal: Time<Galileo> = gps.into_scale().unwrap();

    println!("GPS  → GAL: {} → {}", gps, gal);

    assert_eq!(gps.as_nanos(), gal.as_nanos());

    // GPS → BeiDou (BDT = GPS - 14 seconds)
    let bdt: Time<Beidou> = gps.into_scale().unwrap();

    println!("GPS  → BDT: {} → {}", gps, bdt);

    // GLONASS → UTC (constant shift, no leap seconds)
    let glo = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();
    let utc: Time<Utc> = glo.into_scale().unwrap();

    println!("GLO  → UTC: {} → {}", glo, utc);
}
