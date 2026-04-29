use gnss_time::prelude::*;

fn main() {
    let gps = Time::<Gps>::from_seconds(1_000_000);
    let gal = gps.try_convert::<Galileo>().unwrap();
    let bdt = gps.try_convert::<Beidou>().unwrap();

    println!("GPS : {gps}");
    println!("GAL : {gal}");
    println!("BDT : {bdt}");

    // NOTE: this is the same physical point in time
    assert_eq!(gps.as_nanos(), gal.as_nanos());
}
