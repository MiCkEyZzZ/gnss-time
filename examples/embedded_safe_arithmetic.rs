use gnss_time::{scale::Gps, Duration, DurationParts, GnssTimeError, Time};

fn main() {
    println!("=== Embedded-safe arithmetic demo ===\n");

    // =========================================================
    // Saturating arithmetic (never panics, clamps at bounds)
    // =========================================================

    let t = Time::<Gps>::MAX;
    let safe = t.saturating_add(Duration::from_seconds(1)); // never panic

    assert_eq!(t, safe);
    println!("saturating_add: MAX + 1s = MAX (clamped) ✓");

    let epoch = Time::<Gps>::EPOCH;
    let safe2 = epoch.saturating_sub_duration(Duration::from_nanos(1));

    assert_eq!(safe2, Time::<Gps>::EPOCH);
    println!("saturating_sub: EPOCH - 1ns = EPOCH (clamped) ✓");

    // =========================================================
    // Checked arithmetic (returns Option)
    // =========================================================

    let t2 = Time::<Gps>::from_seconds(1_000_000);

    match t2.checked_add(Duration::from_seconds(500)) {
        Some(result) => {
            println!("checked_add: {} + 500s = {} ✓", t2, result);
        }
        None => {
            println!("checked_add: overflow!");
        }
    }

    match Time::<Gps>::MAX.checked_add(Duration::ONE_NANOSECOND) {
        Some(_) => unreachable!(),
        None => println!("checked_add: MAX + 1ns = None (overflow detected) ✓"),
    }

    // =========================================================
    // Fallible arithmetic (returns GnssTimeError)
    // =========================================================

    match Time::<Gps>::MAX.try_add(Duration::from_seconds(1)) {
        Ok(_) => unreachable!(),
        Err(GnssTimeError::Overflow) => {
            println!("try_add: MAX + 1s = Overflow ✓");
        }
        Err(e) => panic!("unexpected error: {e}"),
    }

    // =========================================================
    // Static initialization (no_std safe)
    // =========================================================

    static REFERENCE: Time<Gps> = Time::<Gps>::EPOCH;
    static WINDOW: Duration = Duration::from_seconds(30);

    println!(
        "\nStatic initializer: reference={REFERENCE}, window={}s ✓",
        WINDOW.as_seconds()
    );

    // =========================================================
    // Panicking arithmetic (safe when bounds are known)
    // =========================================================

    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    let later = gps + Duration::from_seconds(3600);

    println!("\nPanicking add (safe): {gps} + 1h = {later} ✓");

    println!("\nAll safe arithmetic demos passed.");
}
