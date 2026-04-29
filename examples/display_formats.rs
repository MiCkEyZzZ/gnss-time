use gnss_time::prelude::*;

fn main() {
    // GPS / Galileo / BeiDou: Week:TOW format
    // GPS has a dedicated constructor
    let gps = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();

    // For Galileo and BeiDou, create via seconds or nanoseconds
    // Galileo epoch: 1999-08-22, GPS epoch: 1980-01-06
    // Diference: 7168 days = 619_315_200 seconds
    let galileo_nanos = 619_315_200_000_000_000u64 + 432_000_000_000_000u64; // 7168 days + 5 days
    let gal = Time::<Galileo>::from_nanos(galileo_nanos);

    // BeiDou epoch: 2006-01-01, GPS epoch: 1980-01-06
    // Diference: 9492 days = 820_108_800 seconds
    let beidou_nanos = 820_108_800_000_000_000u64 + 432_000_000_000_000u64; // 9492 days + 5 days
    let bdt = Time::<Beidou>::from_nanos(beidou_nanos);

    println!("GPS     : {gps}");
    println!("Galileo : {gal}");
    println!("BeiDou  : {bdt}");
    println!();

    // GLONASS: Day:TOD format
    let glo = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();

    println!("GLONASS : {glo}");
    println!("GLONASS epoch: {}", Time::<Glonass>::EPOCH);
    println!();

    // TAI and UTC: simple format (seconds + nanoseconds)
    let tai = Time::<Tai>::from_seconds(1_000_000_000);
    let utc = Time::<Utc>::from_seconds(1_000_000_000);

    println!("TAI : {tai}");
    println!("UTC : {utc}");
    println!();

    // Demostration of zero-padding
    let gps_early = Time::<Gps>::from_week_tow(1, 1.0).unwrap();
    let glo_early = Time::<Glonass>::from_day_tod(1, 1.0).unwrap();

    println!("GPS early    : {gps_early} (TOW zero-padded to 6 digits)");
    println!("GLONASS early: {glo_early} (TOD zero-padded to 5 digits)");

    // Millisecond precision
    let gps_ms = Time::<Gps>::from_week_tow(100, 0.5).unwrap();

    println!("\nGPS with milliseconds: {gps_ms}");

    // Demostration that Display automatically uses the correct format for each time
    // scale
    let gal_epoch = Time::<Galileo>::EPOCH;
    let bdt_epoch = Time::<Beidou>::EPOCH;

    println!("\nGalileo epoch: {gal_epoch}");
    println!("BeiDou epoch:  {bdt_epoch}");
}
