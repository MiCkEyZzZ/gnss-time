//! Преобразование между различными шкалами времени GNSS с использованием
//! TAI как опорной точки.

use gnss_time::prelude::*;

// TOW для Бэйдоу на 14 секунд меньше, чем у ГПС в один и тот же физический
// момент, потому что эпоха Бэйдоу отстаёт от TAI на 33 с, тогда как ГПС — на 19
// с. Следовательно: BDT = ГПС - 14 с во все моменты времени.
fn main() {
    // ГПС и Галилео имеют одинаковый сдвиг относительно TAI (+19 секунд)
    // Следовательно, они представляют один и тот же физический момент, когда
    // наносекунды равны
    let gps_instant = Time::<Gps>::from_seconds(1_000_000);
    let galileo_instant = gps_instant.try_convert::<Galileo>().unwrap();

    println!("GPS:      {gps_instant}");
    println!("Galileo:  {galileo_instant}");
    println!(
        "Same nanoseconds? {}",
        gps_instant.as_nanos() == galileo_instant.as_nanos()
    );

    // ГПС -> Бэйдоу (BDT = TAI - 33 с, ГПС = TAI - 19 с -> BDT = ГПС - 14 с)
    let bdt_instant = gps_instant.try_convert::<Beidou>().unwrap();

    println!("\nGPS:      {gps_instant}");
    println!("BeiDou:   {bdt_instant}");

    let diff = gps_instant.as_seconds() as i64 - bdt_instant.as_seconds() as i64;

    println!("Difference: {diff} seconds (GPS leads BDT by 14s)");

    // Альтернатива: показать математическое соотношение
    println!(
        "Proof: {}s (GPS) - {}s (BDT) = {}s ✓",
        gps_instant.as_seconds(),
        bdt_instant.as_seconds(),
        diff
    );

    // Явный round-trip через TAI
    let tai = gps_instant.to_tai().unwrap();
    let back_to_gps = Time::<Gps>::from_tai(tai).unwrap();

    assert_eq!(gps_instant, back_to_gps);

    println!("\nRound-trip via TAI works: GPS -> TAI -> GPS");

    // Пример переполнения: ГПС близко к MAX не может быть преобразован в TAI
    let almost_max = Time::<Gps>::from_nanos(u64::MAX - 19_000_000_000);

    match almost_max.to_tai() {
        Ok(_) => println!("Conversion OK"),
        Err(e) => println!("\nOverflow caught: {e}"),
    }

    // Переполнение вниз (underflow): TAI -> ГПС (TAI < 19 с -> отрицательное ГПС)
    let tai_early = Time::<Tai>::from_seconds(10);

    match Time::<Gps>::from_tai(tai_early) {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Underflow caught: {e}"),
    }
}
