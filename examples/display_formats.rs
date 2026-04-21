//! Различные форматы отображения для каждой шкалы времени.

use gnss_time::{Beidou, Galileo, Glonass, Gps, Tai, Time, Utc};

fn main() {
    // ГПС / Галилео / Бэйдоу: формат Неделя:TOW
    // Для ГПС есть специальный конструктор
    let gps = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();

    // Для Галилео и Бэйдоу создаём через секунды или наносекунды
    // Эпоха Галилео: 1999-08-22, эпоха ГПС: 1980-01-06
    // Разница: 7168 дней = 619_315_200 секунд
    let galileo_nanos = 619_315_200_000_000_000u64 + 432_000_000_000_000u64; // 7168 days + 5 days
    let gal = Time::<Galileo>::from_nanos(galileo_nanos);

    // Эпоха Бэйдоу: 2006-01-01, эпоха ГПС: 1980-01-06
    // Разница: 9492 дня = 820_108_800 секунд
    let beidou_nanos = 820_108_800_000_000_000u64 + 432_000_000_000_000u64; // 9492 days + 5 days
    let bdt = Time::<Beidou>::from_nanos(beidou_nanos);

    println!("GPS     : {gps}");
    println!("Galileo : {gal}");
    println!("BeiDou  : {bdt}");
    println!();

    // GLONASS: формат День:TOD
    let glo = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();

    println!("GLONASS : {glo}");
    println!("GLONASS epoch: {}", Time::<Glonass>::EPOCH);
    println!();

    // TAI and UTC: simple format (seconds + nanoseconds)
    let tai = Time::<Tai>::from_seconds(1_000_000_000);
    let utc = Time::<Utc>::from_seconds(1_000_000_000);

    println!("TAI : {tai}");
    println!("UTC : {utc}");
    println!();

    // Демонстрация заполнения нулями
    let gps_early = Time::<Gps>::from_week_tow(1, 1.0).unwrap();
    let glo_early = Time::<Glonass>::from_day_tod(1, 1.0).unwrap();

    println!("GPS early    : {gps_early} (TOW zero-padded to 6 digits)");
    println!("GLONASS early: {glo_early} (TOD zero-padded to 5 digits)");

    // Точность до миллисекунд
    let gps_ms = Time::<Gps>::from_week_tow(100, 0.5).unwrap();

    println!("\nGPS with milliseconds: {gps_ms}");

    // Демонстрация того, что Display автоматически использует правильный формат для
    // каждой шкалы
    let gal_epoch = Time::<Galileo>::EPOCH;
    let bdt_epoch = Time::<Beidou>::EPOCH;

    println!("\nGalileo epoch: {gal_epoch}");
    println!("BeiDou epoch:  {bdt_epoch}");
}
