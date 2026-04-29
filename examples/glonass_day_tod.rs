use gnss_time::prelude::*;

fn main() {
    // GLONASS epoch: 1996-01-01 00:00:00 UTC(SU)
    let epoch = Time::<Glonass>::EPOCH;

    println!("GLONASS epoch: {epoch}");

    // Construct from day number and time of day (in seconds)
    // Day 10512, TOD = 43200 seconds (exactly 12 hourse)
    let t = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();

    println!("GLONASS time: {t}");

    // Extract components back
    println!("Days since epoch: {}", t.day());
    println!("TOD (seconds): {}", t.tod_seconds());

    // Fractional TOD
    let fractional = Time::<Glonass>::from_day_tod(100, 3600.5).unwrap();

    println!("\nFractional seconds example: {fractional}");

    // Invalid TOD (must be < 86400)
    match Time::<Glonass>::from_day_tod(0, 86_400.0) {
        Err(e) => println!("\nInvalid TOD rejected: {e}"),
        _ => panic!("Should have failed!"),
    }

    // Day 0 = epoch
    let day_zero = Time::<Glonass>::from_day_tod(0, 0.0).unwrap();

    assert_eq!(day_zero, Time::<Glonass>::EPOCH);

    println!("\nDay 0, TOD 0 = epoch (confirmed)");
}
