use gnss_time::{DurationParts, GnssTimeError, Gps, IntoScale, ScaleId, Time};

fn main() -> Result<(), GnssTimeError> {
    // =========================================================
    // Runtime conversion routing (SDK layer)
    // =========================================================

    let from_scale = ScaleId::Gps;
    let to_scale = ScaleId::Galileo;

    // Guard for leap-second dependent conversions
    if !from_scale.is_fixed(to_scale) {
        eprintln!("Conversion requires leap seconds; this example only supports fixed offsets.");
        return Ok(());
    }

    // =========================================================
    // Input timestamp (GPS example)
    // =========================================================

    let gps_time = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000,
            nanos: 0,
        },
    )?;

    // =========================================================
    // Type-safe runtime dispatch
    // =========================================================

    let result = match (from_scale, to_scale) {
        (ScaleId::Gps, ScaleId::Galileo) => {
            let gal: Time<gnss_time::Galileo> = gps_time.into_scale()?;
            format!("{gps_time} → {gal}")
        }

        (ScaleId::Gps, ScaleId::Beidou) => {
            let bdt: Time<gnss_time::Beidou> = gps_time.into_scale()?;
            format!("{gps_time} → {bdt}")
        }

        (ScaleId::Gps, ScaleId::Tai) => {
            let tai: Time<gnss_time::Tai> = gps_time.into_scale()?;
            format!("{gps_time} → {tai}")
        }

        _ => unimplemented!("Other scale pairs can be added in the same way"),
    };

    println!("Dynamic conversion: {}", result);

    Ok(())
}
