use gnss_time::prelude::*;

fn main() {
    let _gps = Time::<Gps>::from_seconds(1000);
    let _glo = Time::<Glonass>::from_day_tod(1, 0.0).unwrap();

    // ❌ This will NOT compile:
    // let diff = gps - glo;

    println!("Different time domains cannot be mixed at compile time");
}
