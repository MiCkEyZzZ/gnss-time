use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // GPS / Galileo / BeiDou (Week:Time-of-Week model)
    // =========================================================

    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000,
            nanos: 0,
        },
    )
    .unwrap();

    // Galileo = GPS epoch + offset (fixed relationship)
    let gal = Time::<Galileo>::from_nanos(619_315_200_000_000_000 + 432_000_000_000_000);

    // BeiDou = GPS epoch + different offset
    let bdt = Time::<Beidou>::from_nanos(820_108_800_000_000_000 + 432_000_000_000_000);

    println!("GPS     : {gps}");
    println!("Galileo : {gal}");
    println!("BeiDou  : {bdt}");
    println!();

    // =========================================================
    // GLONASS (Day:Time-of-Day model)
    // =========================================================

    let glo = Time::<Glonass>::from_day_tod(
        10_512,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();

    println!("GLONASS : {glo}");
    println!("Epoch   : {}", Time::<Glonass>::EPOCH);
    println!();

    // =========================================================
    // Atomic / Civil time scales (continuous seconds model)
    // =========================================================

    let tai = Time::<Tai>::from_seconds(1_000_000_000);
    let utc = Time::<Utc>::from_seconds(1_000_000_000);

    println!("TAI : {tai}");
    println!("UTC : {utc}");
    println!();

    // =========================================================
    // Precision & formatting guarantees
    // =========================================================

    let gps_early = Time::<Gps>::from_week_tow(
        1,
        DurationParts {
            seconds: 1,
            nanos: 0,
        },
    )
    .unwrap();

    let glo_early = Time::<Glonass>::from_day_tod(
        1,
        DurationParts {
            seconds: 1,
            nanos: 0,
        },
    )
    .unwrap();

    println!("GPS early    : {gps_early}");
    println!("GLONASS early: {glo_early}");
    println!();

    // =========================================================
    // Sub-second precision
    // =========================================================

    let gps_ms = Time::<Gps>::from_week_tow(
        100,
        DurationParts {
            seconds: 0,
            nanos: 500_000_000,
        },
    )
    .unwrap();

    println!("GPS (ms precision): {gps_ms}");
    println!();

    // =========================================================
    // Epoch consistency check
    // =========================================================

    println!("Galileo epoch: {}", Time::<Galileo>::EPOCH);
    println!("BeiDou epoch : {}", Time::<Beidou>::EPOCH);
}
