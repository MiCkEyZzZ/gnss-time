//! Demonstrates contextual conversions (GPS ↔ UTC) that require leap seconds.

use gnss_time::{prelude::*, ConvertResult};

fn main() {
    // Получаем встроенную таблицу (статическая ссылка)
    let ls = LeapSeconds::builtin();

    // Нормальная конверсия (точная)
    let gps = Time::<Gps>::from_week_tow(2086, 0.0).unwrap();
    let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();

    println!("GPS → UTC (exact): {} → {}", gps, utc);

    // Обратная конверсия
    let gps_back: Time<Gps> = utc.into_scale_with(ls).unwrap();

    assert_eq!(gps, gps_back);

    println!("Round-trip OK");

    // Проверка на неоднозначность в момент вставки leap second
    let gps_ambiguous = Time::<Gps>::from_seconds(1_167_264_018);
    let result: ConvertResult<Time<Utc>> = gps_ambiguous
        .into_scale_with_checked(LeapSeconds::builtin())
        .unwrap();

    match result {
        ConvertResult::Exact(_) => println!("Unexpected exact result"),
        ConvertResult::AmbiguousLeapSecond(utc) => {
            println!("GPS inside leap second → ambiguous, UTC value: {}", utc);
        }
    }
}
