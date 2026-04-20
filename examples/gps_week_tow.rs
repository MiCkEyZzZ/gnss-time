//! Специфичная для GPS функциональность: номер недели и время недели (TOW).

use gnss_time::{scale::Gps, Time};

fn main() {
    // Конструирование из номера недели и TOW (в секундах)
    // Неделя 2345, TOW = 432000 секунд (ровно 5 дней)
    let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();

    println!("ГПС время: {t}");

    // Извлекаем компоненты обратно
    println!("Неделя: {}", t.week());
    println!("Время недели в секундах (TOW): {}", t.tow_seconds());
    println!(
        "Субсекундная часть в наносекундах: {}",
        t.sub_second_nanos()
    );

    // Дробный TOW
    let fractional = Time::<Gps>::from_week_tow(100, 3661.5).unwrap();

    println!("\nС дробными секундами: {fractional}");
    println!(
        "Доля секунды в наносекундах: {}",
        fractional.sub_second_nanos()
    );

    // Некорректный TOW (должен быть < 604800)
    match Time::<Gps>::from_week_tow(0, 604_800.0) {
        Err(e) => println!("\nНекорректный ввод правильно отклонён: {e}"),
        _ => panic!("Должен был завершиться ошибкой!"),
    }

    // Пример переполнения недели GPS (в оборудовании недели идут до 1024 перед сбросом)
    let week_1023 = Time::<Gps>::from_week_tow(1023, 0.0).unwrap();

    println!("\nЭпоха недели 1023: {week_1023}");
}
