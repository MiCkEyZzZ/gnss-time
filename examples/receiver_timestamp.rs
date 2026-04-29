use gnss_time::prelude::*;

fn main() {
    // data from GNSS receiver (e.g., u-blox)
    let week: u16 = 2345;
    let tow: f64 = 432_000.125; // seconds + fractional part

    let t = Time::<Gps>::from_week_tow(week, tow).expect("valid GPS time");

    println!("Receiver timestamp: {t}");
    println!("Week: {}", t.week());
    println!("TOW: {}s", t.tow_seconds());
    println!("Sub-ns: {}", t.sub_second_nanos());
}
