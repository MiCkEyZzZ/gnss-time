use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // Input timestamp (GPS)
    // =========================================================

    let gps = Time::<Gps>::from_week_tow(
        100,
        DurationParts {
            seconds: 1000,
            nanos: 0,
        },
    )
    .unwrap();

    // =========================================================
    // Convert to other constellations
    // =========================================================

    let gal: Time<Galileo> = gps.into_scale().unwrap();
    let bdt: Time<Beidou> = gps.into_scale().unwrap();

    // =========================================================
    // Alignment check (same physical instant?)
    // =========================================================

    let gps_ns = gps.as_nanos();
    let gal_ns = gal.as_nanos();
    let bdt_ns = bdt.as_nanos();

    println!("Alignment check:");
    println!("GPS = GAL? {}", gps_ns == gal_ns);
    println!("GPS = BDT? {}", gps_ns == bdt_ns);
}
