use gnss_time::{DurationParts, Gps, Time};

fn main() {
    // =========================================================
    // 1. Input: raw receiver log (Week, TOW as float seconds)
    // =========================================================
    // Typical format from GNSS receivers / logs

    let log = [(2345, 432000.0), (2345, 432001.0), (2345, 432002.0)];

    // =========================================================
    // 2. Normalize floating-point TOW → (seconds, nanos)
    // =========================================================
    // Important: explicit split avoids precision loss propagation

    for (week, tow) in log {
        let seconds = tow as u64;
        let nanos = ((tow - seconds as f64) * 1_000_000_000.0) as u32;

        // =====================================================
        // 3. Construct strongly-typed timestamp
        // =====================================================

        let t = Time::<Gps>::from_week_tow(week, DurationParts { seconds, nanos }).unwrap();

        println!("{t}");
    }
}
