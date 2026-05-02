use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // Input timestamp (GPS)
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
    // Processing pipeline
    // =========================================================

    let updated = process_timestamp(gps);

    // =========================================================
    // Output
    // =========================================================

    println!("Original : {gps}");
    println!("Processed: {updated}");
}

// =============================================================
// Domain logic (pure, deterministic, no allocation)
// =============================================================

fn process_timestamp(t: Time<Gps>) -> Time<Gps> {
    // Embedded-safe pattern: never panics, clamps on overflow
    t.saturating_add(Duration::from_seconds(1))
}
