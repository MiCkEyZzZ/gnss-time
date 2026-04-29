use gnss_time::prelude::*;

fn main() {
    // Construct from week number and TOW (in seconds)
    // Week 2345, TOW = 432000 seconds (exactly 5 days)
    let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();

    println!("GPS time: {t}");

    // Extract components back
    println!("Week: {}", t.week());
    println!("Time-of-week (TOW) in seconds: {}", t.tow_seconds());
    println!("Sub-second nanoseconds: {}", t.sub_second_nanos());

    // Fractional TOW
    let fractional = Time::<Gps>::from_week_tow(100, 3661.5).unwrap();

    println!("\nWith fractional seconds: {fractional}");
    println!("Sub-second nanoseconds: {}", fractional.sub_second_nanos());

    // Invalid TOW (must be < 604800)
    match Time::<Gps>::from_week_tow(0, 604_800.0) {
        Err(e) => println!("\nInvalid input correctly rejected: {e}"),
        _ => panic!("Should have failed!"),
    }

    // Example of GPS week overflow (in hardware weeks go up to 1024 before
    // rollower)
    let week_1023 = Time::<Gps>::from_week_tow(1023, 0.0).unwrap();

    println!("\nEpoch of week 1023: {week_1023}");
}
