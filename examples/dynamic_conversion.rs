use gnss_time::{GnssTimeError, Gps, IntoScale, ScaleId, Time};

fn main() -> Result<(), GnssTimeError> {
    let from_scale = ScaleId::Gps;
    let to_scale = ScaleId::Galileo;

    if !from_scale.is_fixed(to_scale) {
        eprintln!("Conversion requires leap seconds; this example only supports fixed offsets.");
        return Ok(());
    }

    let gps_time = Time::<Gps>::from_week_tow(2345, 432_000.0)?;

    let result = match (from_scale, to_scale) {
        (ScaleId::Gps, ScaleId::Galileo) => {
            let gal: Time<gnss_time::Galileo> = gps_time.into_scale()?;
            format!("{} → {}", gps_time, gal)
        }
        _ => unimplemented!("Other scale pairs can be added in the same way"),
    };

    println!("Dynamic conversion: {}", result);

    Ok(())
}
