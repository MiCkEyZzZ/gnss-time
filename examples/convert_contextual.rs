use gnss_time::{prelude::*, ConvertResult, DurationParts};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // =========================================================
    // 1. System configuration (leap second model)
    // =========================================================

    let leap_seconds = LeapSeconds::builtin();

    // =========================================================
    // 2. GNSS → UTC (deterministic conversion)
    // =========================================================

    let gps = Time::<Gps>::from_week_tow(
        2086,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    let utc: Time<Utc> = gps.into_scale_with(leap_seconds).unwrap();

    println!("GNSS → Civil time conversion");
    println!("  GPS : {gps}");
    println!("  UTC : {utc}");

    // =========================================================
    // 3. Reverse conversion (consistency check)
    // =========================================================

    let gps_back: Time<Gps> = utc.into_scale_with(leap_seconds).unwrap();

    debug_assert_eq!(gps, gps_back);
    println!("Round-trip consistency: OK");

    // =========================================================
    // 4. Leap second ambiguity handling (edge case)
    // =========================================================

    let ambiguous = Time::<Gps>::from_seconds(1_167_264_018);

    let result: ConvertResult<Time<Utc>> = ambiguous.into_scale_with_checked(leap_seconds).unwrap();

    match result {
        ConvertResult::Exact(utc) => {
            println!("Unexpected exact result: {utc}");
        }

        ConvertResult::AmbiguousLeapSecond(utc) => {
            println!("Leap second window detected:");
            println!("  UTC interpretation: {utc}");
        }
    }

    Ok(())
}
