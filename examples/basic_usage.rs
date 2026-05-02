use gnss_time::prelude::*;

fn main() {
    // ------------------------------------------------------------
    // 1. GNSS epoch (system reference point)
    // ------------------------------------------------------------
    let epoch = Time::<Gps>::EPOCH;
    println!("GPS epoch: {epoch}");

    // ------------------------------------------------------------
    // 2. Receiver-derived timestamp (example: 1 hour after epoch)
    // ------------------------------------------------------------
    let t1 = Time::<Gps>::from_seconds(3600);
    println!("Receiver time: {t1}");

    // ------------------------------------------------------------
    // 3. Time arithmetic (signal processing / correction logic)
    // ------------------------------------------------------------
    let t2 = t1 + Duration::from_seconds(3600);
    println!("Adjusted time (+1h correction): {t2}");

    // ------------------------------------------------------------
    // 4. Time interval extraction (measurement / sync delta)
    // ------------------------------------------------------------
    let delta = t2 - epoch;

    println!(
        "Elapsed since epoch: {}s ({} ns)",
        delta.as_seconds(),
        delta.as_nanos()
    );

    // ------------------------------------------------------------
    // 5. Safety invariants (always guaranteed)
    // ------------------------------------------------------------
    debug_assert!(delta.is_positive());
    debug_assert!(Duration::ZERO.is_zero());

    // ------------------------------------------------------------
    // 6. Embedded-safe arithmetic (no panic behavior)
    // ------------------------------------------------------------
    let clamped = Time::<Gps>::MAX.saturating_add(Duration::from_nanos(1));

    assert_eq!(clamped, Time::<Gps>::MAX);

    println!("Saturating arithmetic: MAX + 1ns → MAX (clamped)");
}
