use gnss_time::prelude::*;

fn main() {
    let t = Time::<Glonass>::from_day_tod(
        10512,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();

    println!("GLONASS time: {t}");
    println!("Day: {}", t.day());
    println!("TOD: {}s", t.tod_seconds());
}
