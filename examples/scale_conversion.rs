use gnss_time::prelude::*;

// BeiDou TOW is 14 seconds behind GPS at the same physical instant,
// because the BeiDou epoch is offset from TAI by 33 s, while GPS is offset
// by 19 s. Therefore: BDT = GPS - 14 s at all times.
fn main() {
    // GPS and Galileo have the same offset relative to TAI (+19 seconds)
    // Therefore, they represent the same physical instant when
    // their nanoseconds are equal
    let gps_instant = Time::<Gps>::from_seconds(1_000_000);
    let galileo_instant = gps_instant.try_convert::<Galileo>().unwrap();

    println!("GPS:      {gps_instant}");
    println!("Galileo:  {galileo_instant}");
    println!(
        "Same nanoseconds? {}",
        gps_instant.as_nanos() == galileo_instant.as_nanos()
    );

    // GPS -> BeiDou (BDT = TAI - 33 s, GPS = TAI - 19 s -> BDT = GPS - 14 s)
    let bdt_instant = gps_instant.try_convert::<Beidou>().unwrap();

    println!("\nGPS:      {gps_instant}");
    println!("BeiDou:   {bdt_instant}");

    let diff = gps_instant.as_seconds() as i64 - bdt_instant.as_seconds() as i64;

    println!("Difference: {diff} seconds (GPS leads BDT by 14s)");

    // Alternative: show the mathematical relation
    println!(
        "Proof: {}s (GPS) - {}s (BDT) = {}s ✓",
        gps_instant.as_seconds(),
        bdt_instant.as_seconds(),
        diff
    );

    // Explicit round-trip via TAI
    let tai = gps_instant.to_tai().unwrap();
    let back_to_gps = Time::<Gps>::from_tai(tai).unwrap();

    assert_eq!(gps_instant, back_to_gps);

    println!("\nRound-trip via TAI works: GPS -> TAI -> GPS");

    // Overflow example: GPS close to MAX cannot be converted to TAI
    let almost_max = Time::<Gps>::from_nanos(u64::MAX - 19_000_000_000);

    match almost_max.to_tai() {
        Ok(_) => println!("Conversion OK"),
        Err(e) => println!("\nOverflow caught: {e}"),
    }

    // Underflow example: TAI -> GPS (TAI < 19 s -> negative GPS)
    let tai_early = Time::<Tai>::from_seconds(10);

    match Time::<Gps>::from_tai(tai_early) {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Underflow caught: {e}"),
    }
}
