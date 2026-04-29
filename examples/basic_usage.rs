use gnss_time::prelude::*;

fn main() {
    // Crate from raw nanoseconds since the GPS epoch (1980-01-06)
    let epoch = Time::<Gps>::EPOCH;

    println!("GPS epoch: {epoch}");

    // Crate from second (helper constructor)
    let one_hour = Time::<Gps>::from_seconds(3600);

    println!("One hour after GPS epoch start: {one_hour}");

    // Add a duration
    let two_hours = one_hour + Duration::from_seconds(3600);

    println!("Two hours: {two_hours}");

    // Diference between two points in time
    let diff = two_hours - epoch;

    println!(
        "Difference: {} seconds = {}ns",
        diff.as_seconds(),
        diff.as_nanos()
    );

    // Check sign and zero value
    assert!(diff.is_positive());
    assert!(Duration::ZERO.is_zero());

    // Saturating arithmetic (never panic)
    let max_safe = Time::<Gps>::MAX.saturating_add(Duration::ONE_NANOSECOND);

    assert_eq!(max_safe, Time::<Gps>::MAX);

    println!("\nSaturating addition works: MAX + 1ns = MAX");
}
