//! Специфичная для GPS функциональность: номер недели и время недели (TOW).

use gnss_time::{scale::Gps, Time};

fn main() {
    // Конструирование из номера недели и TOW (в секундах)
    // Неделя 2345, TOW = 432000 секунд (ровно 5 дней)
    let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();

    println!("GPS time: {t}");

    // Извлекаем компоненты обратно
    println!("Week: {}", t.week());
    println!("Time-of-week (TOW) in seconds: {}", t.tow_seconds());
    println!("Sub-second nanoseconds: {}", t.sub_second_nanos());

    // Дробный TOW
    let fractional = Time::<Gps>::from_week_tow(100, 3661.5).unwrap();

    println!("\nWith fractional seconds: {fractional}");
    println!("Sub-second nanoseconds: {}", fractional.sub_second_nanos());

    // Некорректный TOW (должен быть < 604800)
    match Time::<Gps>::from_week_tow(0, 604_800.0) {
        Err(e) => println!("\nInvalid input correctly rejected: {e}"),
        _ => panic!("Should have failed!"),
    }

    // Пример переполнения недели GPS (в оборудовании недели идут до 1024 перед
    // сбросом)
    let week_1023 = Time::<Gps>::from_week_tow(1023, 0.0).unwrap();

    println!("\nEpoch of week 1023: {week_1023}");
}
