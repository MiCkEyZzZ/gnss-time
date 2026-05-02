use gnss_time::{prelude::*, DurationParts};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ------------------------------------------------------------
    // 1. Incoming receiver timestamp (GPS system time)
    // ------------------------------------------------------------
    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000,
            nanos: 0,
        },
    )
    .unwrap();

    println!("Receiver GPS fix: {}", gps);

    // ------------------------------------------------------------
    // 2. Unified atomic reference (TAI)
    // ------------------------------------------------------------
    let tai: Time<Tai> = gps.into_scale().unwrap();
    println!("Atomic reference (TAI): {}", tai);

    // ------------------------------------------------------------
    // 3. Cross-constellation normalization (Galileo shares GPS time base)
    // ------------------------------------------------------------
    let gal: Time<Galileo> = gps.into_scale().unwrap();
    println!("Galileo aligned: {}", gal);

    debug_assert_eq!(gps.as_nanos(), gal.as_nanos());

    // ------------------------------------------------------------
    // 4. BeiDou conversion (offset-based system)
    // ------------------------------------------------------------
    let bdt: Time<Beidou> = gps.into_scale().unwrap();
    println!("BeiDou aligned: {}", bdt);

    // ------------------------------------------------------------
    // 5. GLONASS → UTC (leap-second dependent system)
    // ------------------------------------------------------------
    let glo = Time::<Glonass>::from_day_tod(
        10_512,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();

    let utc: Time<Utc> = glo.into_scale().unwrap();

    println!("GLONASS fix: {}", glo);
    println!("Civil time (UTC): {}", utc);

    Ok(())
}
