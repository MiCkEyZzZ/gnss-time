use gnss_time::{Glonass, Time};

fn main() {
    // данные из GLONASS эфемерид
    let day = 10512;
    let tod = 43_200.0;

    let t = Time::<Glonass>::from_day_tod(day, tod).unwrap();

    println!("GLONASS time: {t}");
    println!("Day: {}", t.day());
    println!("TOD: {}s", t.tod_seconds());
}
