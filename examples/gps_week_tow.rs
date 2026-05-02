use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // 1. Construct GPS time (Week + Time-of-Week)
    // =========================================================
    // GPS time is represented as:
    // - week number since GPS epoch
    // - time-of-week in seconds (0..604800)

    let t = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000, // 5 days
            nanos: 0,
        },
    )
    .unwrap();

    println!("GPS time: {t}");

    // =========================================================
    // 2. Decompose back into components
    // =========================================================

    println!("Week: {}", t.week());
    println!("Time-of-week (TOW): {}s", t.tow_seconds());
    println!("Sub-second nanoseconds: {}", t.sub_second_nanos());

    // =========================================================
    // 3. Fractional precision (sub-second TOW)
    // =========================================================

    let fractional = Time::<Gps>::from_week_tow(
        100,
        DurationParts {
            seconds: 3_661,
            nanos: 500_000_000,
        },
    )
    .unwrap();

    println!("\nWith fractional seconds: {fractional}");
    println!("Sub-second nanoseconds: {}", fractional.sub_second_nanos());

    // =========================================================
    // 4. Domain validation (TOW bounds)
    // =========================================================
    // TOW must be in range [0, 604800)

    match Time::<Gps>::from_week_tow(
        0,
        DurationParts {
            seconds: 604_800,
            nanos: 0,
        },
    ) {
        Ok(_) => unreachable!(),
        Err(e) => println!("\nInvalid input correctly rejected: {e}"),
    }

    // =========================================================
    // 5. Week rollover behavior (GPS specification context)
    // =========================================================
    // GPS week counter is historically 10-bit (0..1023)

    let week_1023 = Time::<Gps>::from_week_tow(
        1023,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    println!("\nEpoch of week 1023: {week_1023}");
}
