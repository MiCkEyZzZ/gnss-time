use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // Receiver timestamp (GPS Week + TOW)
    // =========================================================

    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000,
            nanos: 125_000_000,
        },
    )
    .expect("valid GPS timestamp");

    // =========================================================
    // Structured access (decode components)
    // =========================================================

    let week = gps.week();
    let tow = gps.tow_seconds();
    let sub_ns = gps.sub_second_nanos();

    // =========================================================
    // Output
    // =========================================================

    println!("Receiver timestamp: {gps}");
    println!("Week: {week}");
    println!("TOW: {tow}s");
    println!("Sub-ns: {sub_ns}");
}
