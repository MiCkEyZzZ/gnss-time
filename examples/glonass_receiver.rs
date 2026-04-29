use gnss_time::prelude::*;

fn main() {
    // data from GLONASS ephemeris
    let day = 10512;
    let tod = 43_200.0;

    let t = Time::<Glonass>::from_day_tod(day, tod).unwrap();

    println!("GLONASS time: {t}");
    println!("Day: {}", t.day());
    println!("TOD: {}s", t.tod_seconds());
}
