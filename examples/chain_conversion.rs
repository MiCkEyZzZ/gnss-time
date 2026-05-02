use gnss_time::{Beidou, LeapSeconds, Time};

fn main() {
    // ------------------------------------------------------------
    // 1. System configuration (leap second model)
    // ------------------------------------------------------------
    let leap_seconds = LeapSeconds::builtin();

    // ------------------------------------------------------------
    // 2. Incoming receiver timestamp (BeiDou domain)
    // ------------------------------------------------------------
    let bdt = Time::<Beidou>::from_seconds(2_000_000_000);

    println!("Incoming BeiDou fix: {}", bdt);

    // ------------------------------------------------------------
    // 3. GNSS normalization pipeline (BeiDou → GPS → GLONASS → UTC → TAI)
    // ------------------------------------------------------------
    let result = gnss_time::matrix::beidou_via_gps_to_glonass_via_utc(bdt, &leap_seconds);

    match result {
        Ok(chain) => {
            println!("\nNormalized GNSS time chain:");
            println!("  GPS     : {}", chain.gps);
            println!("  GLONASS : {}", chain.glonass);
            println!("  UTC     : {}", chain.utc);
            println!("  TAI     : {}", chain.tai);
        }

        Err(e) => {
            println!("GNSS conversion failed: {e}");
        }
    }
}
