use gnss_time::{Beidou, Galileo, Gps, Time};

fn main() {
    let gps = Time::<Gps>::from_week_tow(100, 1000.0).unwrap();

    let gal = gps.try_convert::<Galileo>().unwrap();
    let bdt = gps.try_convert::<Beidou>().unwrap();

    println!("Alignment check:");
    println!("GPS = GAL? {}", gps.as_nanos() == gal.as_nanos());
    println!("GPS = BDT? {}", gps.as_nanos() == bdt.as_nanos());
}
