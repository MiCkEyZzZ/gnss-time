use gnss_time::{Beidou, Galileo, Gps, Time};

fn main() {
    let gps = Time::<Gps>::from_seconds(1_000_000);
    let gal = gps.try_convert::<Galileo>().unwrap();
    let bdt = gps.try_convert::<Beidou>().unwrap();

    println!("GPS : {gps}");
    println!("GAL : {gal}");
    println!("BDT : {bdt}");

    // важно: это один физический момент времени
    assert_eq!(gps.as_nanos(), gal.as_nanos());
}
