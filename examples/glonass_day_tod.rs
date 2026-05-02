use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // GLONASS time system (Day + Time-of-Day)
    // =========================================================

    // GLONASS epoch: 1996-01-01 00:00:00 UTC(SU)
    let epoch = Time::<Glonass>::EPOCH;

    println!("GLONASS epoch: {epoch}");

    // =========================================================
    // Construct from Day + Time-of-Day (TOD)
    // =========================================================

    // Day 10512, TOD = 43200 seconds (12 hours)
    let t = Time::<Glonass>::from_day_tod(
        10_512,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();

    println!("GLONASS time: {t}");

    // =========================================================
    // Component extraction
    // =========================================================

    println!("Days since epoch: {}", t.day());
    println!("TOD (seconds): {}", t.tod_seconds());

    // =========================================================
    // Fractional TOD precision (nanoseconds supported)
    // =========================================================

    let fractional = Time::<Glonass>::from_day_tod(
        100,
        DurationParts {
            seconds: 3600,
            nanos: 500_000_000,
        },
    )
    .unwrap();

    println!("\nFractional seconds example: {fractional}");

    // =========================================================
    // Input validation (TOD must be < 86_400)
    // =========================================================

    match Time::<Glonass>::from_day_tod(
        0,
        DurationParts {
            seconds: 86_400,
            nanos: 0,
        },
    ) {
        Err(e) => println!("\nInvalid TOD rejected: {e}"),
        _ => panic!("Should have failed!"),
    }

    // =========================================================
    // Identity check (epoch correctness)
    // =========================================================

    let day_zero = Time::<Glonass>::from_day_tod(
        0,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(day_zero, Time::<Glonass>::EPOCH);

    println!("\nDay 0, TOD 0 = epoch (confirmed)");
}
