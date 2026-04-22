//! Специфичная для ГЛОНАСС функциональность: номер дня и время суток (TOD).

use gnss_time::prelude::*;

fn main() {
    // Эпоха ГЛОНАСС: 1996-01-01 00:00:00 UTC(SU)
    let epoch = Time::<Glonass>::EPOCH;

    println!("GLONASS epoch: {epoch}");

    // Конструирование из номера дня и времени суток (в секундах)
    // День 10512, TOD = 43200 секунд (ровно 12 часов)
    let t = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();

    println!("GLONASS time: {t}");

    // Извлекаем компоненты обратно
    println!("Days since epoch: {}", t.day());
    println!("TOD (seconds): {}", t.tod_seconds());

    // Дробный TOD
    let fractional = Time::<Glonass>::from_day_tod(100, 3600.5).unwrap();

    println!("\nFractional seconds example: {fractional}");

    // Некорректный TOD (должен быть < 86400)
    match Time::<Glonass>::from_day_tod(0, 86_400.0) {
        Err(e) => println!("\nInvalid TOD rejected: {e}"),
        _ => panic!("Should have failed!"),
    }

    // День 0 = эпоха
    let day_zero = Time::<Glonass>::from_day_tod(0, 0.0).unwrap();

    assert_eq!(day_zero, Time::<Glonass>::EPOCH);

    println!("\nDay 0, TOD 0 = epoch (confirmed)");
}
