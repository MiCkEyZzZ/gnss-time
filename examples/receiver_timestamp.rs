use gnss_time::{Gps, Time};

fn main() {
    // данные из GNSS приёмника (например u-blox)
    let week: u16 = 2345;
    let tow: f64 = 432_000.125; // секунд + дробная часть

    let t = Time::<Gps>::from_week_tow(week, tow).expect("valid GPS time");

    println!("Receiver timestamp: {t}");
    println!("Week: {}", t.week());
    println!("TOW: {} s", t.tow_seconds());
    println!("Sub-ns: {}", t.sub_second_nanos());
}
