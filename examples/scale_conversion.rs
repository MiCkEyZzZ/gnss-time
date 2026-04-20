//! Преобразование между различными шкалами времени GNSS с использованием
//! TAI как опорной точки.

use gnss_time::{Beidou, Galileo, Gps, Tai, Time};

// TOW для Бэйдоу на 14 секунд меньше, чем у ГПС в один и тот же физический момент,
// потому что эпоха Бэйдоу отстаёт от TAI на 33 с, тогда как ГПС — на 19 с.
// Следовательно: BDT = ГПС - 14 с во все моменты времени.
fn main() {
    // ГПС и Галилео имеют одинаковый сдвиг относительно TAI (+19 секунд)
    // Следовательно, они представляют один и тот же физический момент, когда наносекунды равны
    let gps_instant = Time::<Gps>::from_seconds(1_000_000);
    let galileo_instant = gps_instant.try_convert::<Galileo>().unwrap();

    println!("ГПС:      {gps_instant}");
    println!("Галилео:  {galileo_instant}");
    println!(
        "Одинаковые наносекунды? {}",
        gps_instant.as_nanos() == galileo_instant.as_nanos()
    );

    // ГПС -> Бэйдоу (BDT = TAI - 33 с, ГПС = TAI - 19 с -> BDT = ГПС - 14 с)
    let bdt_instant = gps_instant.try_convert::<Beidou>().unwrap();

    println!("\nГПС:      {gps_instant}");
    println!("Бэйдоу:   {bdt_instant}");

    let diff = gps_instant.as_seconds() as i64 - bdt_instant.as_seconds() as i64;

    println!("Разница: {diff} секунд (ГПС опережает BDT на 14 с)");

    // Альтернатива: показать математическое соотношение
    println!(
        "Доказательство: {}с (ГПС) - {}с (BDT) = {}с ✓",
        gps_instant.as_seconds(),
        bdt_instant.as_seconds(),
        diff
    );

    // Явный round-trip через TAI
    let tai = gps_instant.to_tai().unwrap();
    let back_to_gps = Time::<Gps>::from_tai(tai).unwrap();

    assert_eq!(gps_instant, back_to_gps);

    println!("\nСквозное преобразование через TAI работает: ГПС → TAI → ГПС");

    // Пример переполнения: ГПС близко к MAX не может быть преобразован в TAI
    let almost_max = Time::<Gps>::from_nanos(u64::MAX - 19_000_000_000);

    match almost_max.to_tai() {
        Ok(_) => println!("Преобразование успешно"),
        Err(e) => println!("\nПереполнение перехвачено: {e}"),
    }

    // Переполнение вниз (underflow): TAI -> ГПС (TAI < 19 с -> отрицательное ГПС)
    let tai_early = Time::<Tai>::from_seconds(10);

    match Time::<Gps>::from_tai(tai_early) {
        Ok(_) => println!("Неожиданный успех"),
        Err(e) => println!("Перехвачено исчерпание: {e}"),
    }
}
