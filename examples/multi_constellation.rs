use gnss_time::prelude::*;

fn main() {
    // =========================================================
    // 1. Input: reference time (GPS as canonical source)
    // =========================================================

    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000,
            nanos: 0,
        },
    )
    .unwrap();

    // =========================================================
    // 2. Normalize to other constellations
    // =========================================================
    // Fixed-offset systems (no leap seconds required)

    let gal: Time<Galileo> = gps.into_scale().unwrap();
    let bdt: Time<Beidou> = gps.into_scale().unwrap();

    // Contextual system (requires leap second table)

    let ls = LeapSeconds::builtin();
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();

    // =========================================================
    // 3. Output (aligned physical instant across systems)
    // =========================================================

    println!("GPS : {gps}");
    println!("GAL : {gal}");
    println!("BDT : {bdt}");
    println!("GLO : {glo}");
}
