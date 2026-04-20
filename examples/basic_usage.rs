//! Основные функции `gnss-time`: создание моментов времени, арифметические
//! операции и вычисление разностей.

use gnss_time::{scale::Gps, Duration, Time};

fn main() {
    // Создаём из исходных наносекунд с начала эпохи GPS (1980-01-06)
    let epoch = Time::<Gps>::EPOCH;

    println!("ГПС эпоха: {epoch}");

    // Создание из секунд (вспомогательный конструктор)
    let one_hour = Time::<Gps>::from_seconds(3600);

    println!("Один час после начала эпохи: {one_hour}");

    // Добавляем продолжительность
    let two_hours = one_hour + Duration::from_seconds(3600);

    println!("Два часа: {two_hours}");

    // Разница между двумя моментами времени
    let diff = two_hours - epoch;

    println!(
        "Разница: {} секунд = {} нс",
        diff.as_seconds(),
        diff.as_nanos()
    );

    // Проверяем знак и нулевое значение
    assert!(diff.is_positive());
    assert!(Duration::ZERO.is_zero());

    // Арифметика с насыщением (никогда не вызывает панику)
    let max_safe = Time::<Gps>::MAX.saturating_add(Duration::ONE_NANOSECOND);

    assert_eq!(max_safe, Time::<Gps>::MAX);

    println!("\nНасыщающее сложение работает: MAX + 1нс = MAX");
}
