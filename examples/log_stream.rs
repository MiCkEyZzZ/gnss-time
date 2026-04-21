use gnss_time::{Gps, Time};

fn main() {
    let log = vec![(2345, 432000.0), (2345, 432001.0), (2345, 432002.0)];

    for (week, tow) in log {
        let t = Time::<Gps>::from_week_tow(week, tow).unwrap();

        println!("{t}");
    }
}
