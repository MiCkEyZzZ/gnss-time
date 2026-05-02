use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // Input: raw receiver timestamp (GPS time)
    // =========================================================

    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    // =========================================================
    // 1. Safe correction (fallible arithmetic)
    // =========================================================
    // Used when overflow must be explicitly handled

    let corrected = gps
        .checked_add(Duration::from_seconds(1))
        .expect("1s correction is within bounds");

    println!("Original:  {gps}");
    println!("Corrected: {corrected}");

    // =========================================================
    // 2. Embedded-safe saturation model
    // =========================================================
    // Never panics, clamps at MAX

    let saturated = Time::<Gps>::MAX.saturating_add(Duration::from_seconds(1));

    println!("Saturated: {saturated}");
}
