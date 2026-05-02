use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // Same physical instant across different GNSS scales
    // =========================================================

    let gps = Time::<Gps>::from_seconds(1_000_000);

    let gal: Time<Galileo> = gps.into_scale().unwrap();

    println!("GPS:      {gps}");
    println!("Galileo:  {gal}");
    println!("Same nanoseconds? {}", gps.as_nanos() == gal.as_nanos());

    // =========================================================
    // GPS ↔ BeiDou offset (constant 14 seconds)
    // =========================================================

    let bdt: Time<Beidou> = gps.into_scale().unwrap();

    println!("\nGPS:    {gps}");
    println!("BDT:    {bdt}");

    let diff = gps.as_seconds() as i64 - bdt.as_seconds() as i64;

    println!("Difference: {diff} seconds (GPS leads BDT by 14s)");

    println!(
        "Proof: {}s (GPS) - {}s (BDT) = {}s ✓",
        gps.as_seconds(),
        bdt.as_seconds(),
        diff
    );

    // =========================================================
    // Deterministic conversion via TAI (pivot)
    // =========================================================

    let tai: Time<Tai> = gps.into_scale().unwrap();
    let gps_back: Time<Gps> = tai.into_scale().unwrap();

    assert_eq!(gps, gps_back);

    println!("\nRound-trip via TAI works: GPS -> TAI -> GPS");

    // =========================================================
    // Overflow protection (upper bound)
    // =========================================================

    let near_max = Time::<Gps>::from_nanos(u64::MAX - 19_000_000_000);

    match near_max.into_scale() {
        Ok(tai) => {
            let _: Time<Tai> = tai;
            println!("Conversion OK");
        }
        Err(e) => println!("\nOverflow caught: {e}"),
    }

    // =========================================================
    // Underflow protection (lower bound)
    // =========================================================

    let tai_early = Time::<Tai>::from_seconds(10);

    match tai_early.into_scale() {
        Ok(gps) => {
            let _: Time<Gps> = gps;
            println!("Unexpected success");
        }
        Err(e) => println!("Underflow caught: {e}"),
    }
}
