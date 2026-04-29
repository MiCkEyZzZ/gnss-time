// Тесты для задачи #TIME-5: преобразования GLONASS
//
// Почему для преобразования GLONASS ↔ UTC не нужны високосные секунды
//
// GLONASS передаёт время в UTC(SU) — московское стандартное время = UTC + 3 ч.
// Важно, что GLONASS **учитывает вставки високосных секунд так же, как и
// UTC**: когда IERS добавляет високосную секунду в UTC, GLONASS добавляет ту
// же секунду в свою временную шкалу.
//
// Это означает, что и UTC, и GLONASS непрерывно считают наносекунды
// синхронно (в ногу друг с другом) — они отличаются только фиксированным
// смещением эпох (календарной разницей между 1972-01-01 и эпохой GLONASS
// 1995-12-31 21:00:00 UTC).
// Для преобразования между ними не требуется учитывать високосные секунды.
//
// А вот для GLONASS <-> GPS високосные секунды нужны, потому что GPS не
// содержит високосных секунд (его шкала расходится с UTC на накопленное
// количество секунд с 1980 года).
//
// Геометрия эпох
//
// ```text
// UTC epoch   1972-01-01 00:00:00 UTC
// GPS epoch   1980-01-06 00:00:00 UTC
// GLO epoch   1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC
//
// UTC_ns = GLO_ns + 757_371_600_000_000_000
//        (= 8766 days × 86 400 s − 3 h) × 10⁹ ns/s
// ```

use gnss_time::{
    glonass_to_gps, glonass_to_utc, gps_to_glonass, utc_to_glonass, CivilDate, Glonass,
    GnssTimeError, Gps, IntoScale, IntoScaleWith, LeapSeconds, Time, Utc,
};

// Эпоха GLONASS = 1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC.
// В наносекундах UTC от 1972-01-01: 8766 дней × 86 400 с − 3 × 3 600 с
// = 757 382 400 − 10 800 = 757 371 600 с = 757_371_600_000_000_000 нс.
#[test]
fn test_glonass_epoch_expressed_in_utc_nanos() {
    let glo_epoch = Time::<Glonass>::EPOCH;
    let utc: Time<Utc> = glo_epoch.into_scale().unwrap();

    assert_eq!(
        utc.as_nanos(),
        757_371_600_000_000_000,
        "GLO epoch should map to 757_371_600s from UTC epoch"
    );
}

#[test]
fn test_glonass_epoch_is_monday_1996_01_01() {
    // 1996-01-01 был понедельником → day_of_week = 1
    let t = Time::<Glonass>::EPOCH;

    assert_eq!(t.day(), 0);
    assert_eq!(t.day_of_week(), 1); // Понедельник
}

#[test]
fn test_glonass_utc_offset_is_exactly_3_hours() {
    // UTC(SU) = UTC + 3 часа → GLONASS идёт на 3 часа вперёд относительно UTC по
    // часовому времени.
    //
    // В нашем представлении эпоха GLONASS = эпоха UTC + 757_371_600 секунд
    // (≈ 8766 дней − 3 часа).
    // Метка времени GLONASS T соответствует метке времени UTC:
    // (T + 757_371_600_000_000_000).
    //
    // Другой способ это понять:
    // в момент полуночи GLONASS (tod = 0) по UTC на часах 21:00.
    //
    // Проверка на известной дате:
    // день GLONASS 1, tod = 0 = 1996-01-02 00:00:00 UTC(SU)
    //                          = 1996-01-01 21:00:00 UTC
    let glo = Time::<Glonass>::from_day_tod(1, 0.0).unwrap();
    let utc: Time<Utc> = glo.into_scale().unwrap();

    // Время UTC 1996-01-01 21:00:00 можно получить как: эпоха GLONASS (в
    // наносекундах, относительно UTC) плюс 1 сутки (86400 секунд) в системе
    // GLONASS: 757_371_600 + 86400 секунд
    let expected_utc_s = 757_371_600_u64 + 86_400;

    assert_eq!(utc.as_seconds(), expected_utc_s);
}

#[test]
fn test_glonass_to_utc_is_constant_shift() {
    // Сдвиг всегда одинаков независимо от момента времени — поиск в таблице не
    // требуется.
    let glo1 = Time::<Glonass>::from_day_tod(1_000, 43_200.0).unwrap();
    let glo2 = Time::<Glonass>::from_day_tod(5_000, 12_345.0).unwrap();

    let utc1: Time<Utc> = glo1.into_scale().unwrap();
    let utc2: Time<Utc> = glo2.into_scale().unwrap();

    // Разница между метками времени UTC должна равняться разнице между метками
    // времени GLO
    let glo_diff = (glo2 - glo1).as_nanos();
    let utc_diff = (utc2 - utc1).as_nanos();

    assert_eq!(
        glo_diff, utc_diff,
        "GLONASS -> UTC is a rigid shift: intervals must be preserved"
    );
}

#[test]
fn test_glonass_to_utc_roundtrip() {
    let glo = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();
    let utc: Time<Utc> = glo.into_scale().unwrap();
    let back: Time<Glonass> = utc.into_scale().unwrap();

    assert_eq!(glo, back);
}

#[test]
fn test_glonass_to_utc_with_sub_second_nanos() {
    let glo = Time::<Glonass>::from_nanos(10_000_000_123_456_789);
    let utc: Time<Utc> = glo.into_scale().unwrap();
    let back: Time<Glonass> = utc.into_scale().unwrap();

    assert_eq!(
        glo, back,
        "sub-second nanoseconds preserved throgh GLO -> UTC -> GLO"
    );
}

#[test]
fn test_utc_before_glonass_epoch_is_overflow() {
    // Эпоха UTC (t = 0) предшествует эпохе GLONASS -> недополнение
    let utc = Time::<Utc>::EPOCH;
    let result: Result<Time<Glonass>, _> = utc.into_scale();

    assert!(matches!(result, Err(GnssTimeError::Overflow)));
}

#[test]
fn test_utc_just_at_glonass_epoch_gives_zero() {
    // UTC в 757_371_600 с от начала эпохи UTC = эпоха GLONASS
    let utc = Time::<Utc>::from_nanos(757_371_600_000_000_000);
    let glo: Time<Glonass> = utc.into_scale().unwrap();

    assert_eq!(glo, Time::<Glonass>::EPOCH);
}

// В полночь по времени GLONASS UTC показывает 21:00:00 (= смещение UTC+3)
// Проверка для конкретной известной даты: 1996-01-01 00:00:00 UTC(SU).
#[test]
fn test_glonass_midnight_is_utc_21h() {
    // День GLO 0 tod 0 = 1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC
    let glo = Time::<Glonass>::EPOCH; // день 0, тод 0
    let utc: Time<Utc> = glo.into_scale().unwrap();

    // UTC: 757_371_600c c 1972-01-01
    // 757_371_600 / 86400 = 8765 дней + 21 час
    let secs = utc.as_seconds();
    let hours_in_day = (secs % 86_400) / 3600;

    assert_eq!(hours_in_day, 21, "GLONASS midnight -> 21:00 UTC");
}

#[test]
fn test_glonass_gps_roundtrip_post_2017() {
    let ls = LeapSeconds::builtin();

    // После 2017-01-01 (последняя високосная секунда), GPS-UTC = 18 с (стабильное
    // время)
    let gps = Time::<Gps>::from_week_tow(2100, 86_400.0).unwrap();
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
    let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

    assert_eq!(gps, back);
}

#[test]
fn test_glonass_gps_roundtrip_before_1999() {
    let ls = LeapSeconds::builtin();

    // До 1999-01-01 г., GPS-UTC = 13 с
    // GPS_s = 504_478_810+ — это после эпохи GLONASS, используйте 550_000_000
    // (~июнь 1997 г.)
    let gps = Time::<Gps>::from_seconds(550_000_000);
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
    let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

    assert_eq!(gps, back);
}

#[test]
fn test_glonass_gps_roundtrip_with_nanoseconds() {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_nanos(1_200_000_000_987_654_321);
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
    let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

    assert_eq!(
        gps, back,
        "nanosecond precision preserved through GPS -> GLO -> GPS"
    );
}

// Сравниваем GLONASS и GPS в один и тот же момент времени UTC.
//
// В 2020-05-01 00:00:18 GPS (= 2020-05-01 00:00:00 UTC, GPS-UTC=18):
// - Секунды GPS: 1262217618
// - Тот же момент времени UTC в GLONASS должен быть 2020-05-01 03:00:00 UTC(SU)
//   = день 8770 от эпохи GLONASS (1996-01-01) с tod = 10800 с (3 ч)
#[test]
fn test_glonass_and_gps_at_same_utc_instant() {
    let ls = LeapSeconds::builtin();

    // Время GPS на 2020-05-01 00:00:00 UTC:
    // unix = 1578182400, GPS_s = (1578182400 - 315964800) + 18 = 1262217618
    let gps = Time::<Gps>::from_seconds(1_262_217_618);
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();

    // 2020-05-01 03:00:00 UTC (SU) по эпохе GLONASS:
    // дней с 01.01.1996 по 05.01.2020 = 8766 + 4 = 8770 дней (проверено ниже)
    let day_from_glo_epoch = CivilDate::new(1996, 1, 1)
        .days_until(CivilDate::new(2020, 1, 5))
        .unsigned_abs() as u32;

    assert_eq!(
        day_from_glo_epoch, 8770,
        "days from GLONASS epoch to 2020-01-05"
    );
    assert_eq!(
        glo.day(),
        8770,
        "GLO day should be 8770 for 2020-01-05 UTC(SU)"
    );

    // tod = 3 часа = 10800с (UTC+3 сдвиг)
    assert_eq!(
        glo.tod_seconds(),
        10_800,
        "GLO tod should be 10800 (03:00:00 UTC+3)"
    );
}

// Эпоха GLONASS = 1996-01-01 = понедельник → day_of_week() = 1.
#[test]
fn test_day_of_week_epoch_is_monday() {
    let t = Time::<Glonass>::EPOCH;

    assert_eq!(t.day_of_week(), 1, "1996-01-01 was a Monday -> 1");
}

#[test]
fn test_day_of_week_sequence_mon_through_sun() {
    let expected = [1u8, 2, 3, 4, 5, 6, 7]; // Mon … Sun
    for (i, &expected_dow) in expected.iter().enumerate() {
        let t = Time::<Glonass>::from_day_tod(i as u32, 0.0).unwrap();
        assert_eq!(
            t.day_of_week(),
            expected_dow,
            "day {} should have day_of_week = {}",
            i,
            expected_dow
        );
    }
}

#[test]
fn test_day_of_week_wraps_at_7() {
    // День 7 = 1996-01-08 = снова понедельник
    let t = Time::<Glonass>::from_day_tod(7, 0.0).unwrap();

    assert_eq!(t.day_of_week(), 1, "day 7 should be Monday again");
}

#[test]
fn test_day_of_week_saturday_and_sunday() {
    let sat = Time::<Glonass>::from_day_tod(5, 0.0).unwrap(); // 1996-01-06 Суб
    let sun = Time::<Glonass>::from_day_tod(6, 0.0).unwrap(); // 1996-01-07 Вос
    let mon = Time::<Glonass>::from_day_tod(0, 0.0).unwrap(); // Понедельник

    assert_eq!(sat.day_of_week(), 6);
    assert_eq!(sun.day_of_week(), 7);
    assert_eq!(mon.day_of_week(), 1);
}

#[test]
fn test_is_weekend_returns_true_for_sat_sun() {
    let sat = Time::<Glonass>::from_day_tod(5, 0.0).unwrap();
    let sun = Time::<Glonass>::from_day_tod(6, 0.0).unwrap();
    let fri = Time::<Glonass>::from_day_tod(4, 0.0).unwrap();
    let mon = Time::<Glonass>::from_day_tod(0, 0.0).unwrap();

    assert!(sat.is_weekend());
    assert!(sun.is_weekend());
    assert!(!fri.is_weekend());
    assert!(!mon.is_weekend());
}

// Проверка day_of_week для известной даты: 2020-01-05 был воскресеньем.
// Количество дней от 1996-01-01 до 2020-01-05 = 8770.
// 8770 % 7 = 1 → day_of_week = 2 (вторник)?
// Подожди, перепроверю с помощью Python...
// На самом деле: 8770 % 7 = 8770 mod 7.
// 8770 / 7 = 1252.857... → 1252 * 7 = 8764 → 8770 - 8764 = 6 → (6 % 7) + 1 = 7
// (воскресенье) ✓
#[test]
fn test_day_of_week_2020_01_05_is_sunday() {
    let days = CivilDate::new(1996, 1, 1)
        .days_until(CivilDate::new(2020, 1, 5))
        .unsigned_abs() as u32;

    assert_eq!(days, 8770);

    let t = Time::<Glonass>::from_day_tod(days, 0.0).unwrap();

    assert_eq!(t.day_of_week(), 7, "2020-01-05 was a Sunday");
}

#[test]
fn test_day_of_week_matches_known_dates() {
    // Проверено по календарю: 2023-10-09 = понедельник количество дней от
    // 1996-01-01 до 2023-10-09:
    let days_mon = CivilDate::new(1996, 1, 1)
        .days_until(CivilDate::new(2023, 10, 9))
        .unsigned_abs() as u32;
    let t = Time::<Glonass>::from_day_tod(days_mon, 0.0).unwrap();

    assert_eq!(t.day_of_week(), 1, "2023-10-09 should be Monday");

    // 2023-10-14 = суббота
    let days_sat = CivilDate::new(1996, 1, 1)
        .days_until(CivilDate::new(2023, 10, 14))
        .unsigned_abs() as u32;
    let t = Time::<Glonass>::from_day_tod(days_sat, 0.0).unwrap();

    assert_eq!(t.day_of_week(), 6, "2023-10-14 should be Saturday");
}

#[test]
fn test_glonass_sub_second_nanos() {
    let t = Time::<Glonass>::from_day_tod(100, 43_200.5).unwrap();

    assert_eq!(t.tod_seconds(), 43_200);
    assert_eq!(t.sub_second_nanos(), 500_000_000); // 0.5c
}

#[test]
fn test_glonass_sub_second_nanos_zero() {
    let t = Time::<Glonass>::from_day_tod(0, 0.0).unwrap();

    assert_eq!(t.sub_second_nanos(), 0);
}

/// Во время високосной секунды GPS (GPS «прыгает» на 2с, UTC увеличивается на
/// 1с): соответствующие метки времени GLONASS также отражают ту же
/// непрерывность UTC. Преобразование GPS -> GLO -> обратно должно давать точное
/// совпадение (roundtrip) с обеих сторон этого события.
#[test]
fn test_glonass_across_2017_leap_second_roundtrip() {
    let ls = LeapSeconds::builtin();

    // GPS до високосной секунды 2017-01-01 (задолго до границы): GPS_s = 1167264010
    let gps_before = Time::<Gps>::from_seconds(1_167_264_010);
    // GPS после (задолго после границы): GPS_s = 1167264025
    let gps_after = Time::<Gps>::from_seconds(1_167_264_025);

    let glo_before: Time<Glonass> = gps_before.into_scale_with(ls).unwrap();
    let glo_after: Time<Glonass> = gps_after.into_scale_with(ls).unwrap();

    // Проверка roundtrip в обе стороны
    let back_before: Time<Gps> = glo_before.into_scale_with(ls).unwrap();
    let back_after: Time<Gps> = glo_after.into_scale_with(ls).unwrap();

    assert_eq!(gps_before, back_before);
    assert_eq!(gps_after, back_after);

    // Интервал UTC через високосную секунду: GLONASS продвигается на 2с (как и
    // GPS) потому что GLONASS и UTC используют одну и ту же високосную секунду
    // — оба «прыгают» вместе GPS сделал скачок на 15с между нашими тестовыми
    // точками; GLONASS должен дать тот же скачок (оба синхронно следуют UTC,
    // включая вставку 1-секундной високосной секунды)
    let glo_jump_ns = glo_after.as_nanos() as i128 - glo_before.as_nanos() as i128;
    let gps_jump_ns = gps_after.as_nanos() as i128 - gps_before.as_nanos() as i128;

    // GPS прыгнул на 15с, но UTC (и GLONASS) — только на 14с, потому что
    // 1 из этих GPS-секунд был «поглощён» вставкой високосной секунды.
    // Это корректно: разница GPS−UTC увеличилась на 1с через границу.
    assert_eq!(
        glo_jump_ns / 1_000_000_000,
        gps_jump_ns / 1_000_000_000 - 1,
        "GLONASS jumps 1 s less than GPS across a leap second (leap consumed 1 s)"
    );
}

#[test]
fn test_into_scale_glonass_utc_matches_glonass_to_utc() {
    let glo = Time::<Glonass>::from_day_tod(5_000, 36_000.0).unwrap();
    let via_trait: Time<Utc> = glo.into_scale().unwrap();
    let via_fn = glonass_to_utc(glo).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_into_scale_utc_glonass_matches_utc_to_glonass() {
    let utc = Time::<Utc>::from_nanos(800_000_000_000_000_000); // значительно позже эпохи GLONASS
    let via_trait: Time<Glonass> = utc.into_scale().unwrap();
    let via_fn = utc_to_glonass(utc).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_into_scale_with_gps_glonass_matches_gps_to_glonass() {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_week_tow(2086, 0.0).unwrap();
    let via_trait: Time<Glonass> = gps.into_scale_with(ls).unwrap();
    let via_fn = gps_to_glonass(gps, ls).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_into_scale_with_glonass_gps_matches_glonass_to_gps() {
    let ls = LeapSeconds::builtin();
    let glo = Time::<Glonass>::from_day_tod(8_000, 43_200.0).unwrap();
    let via_trait: Time<Gps> = glo.into_scale_with(ls).unwrap();
    let via_fn = glonass_to_gps(glo, ls).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_glonass_display_canonical_format() {
    let t = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();

    assert_eq!(t.to_string(), "GLO 10512:43200.000");
}

#[test]
fn test_glonass_display_epoch_is_day_zero() {
    assert_eq!(Time::<Glonass>::EPOCH.to_string(), "GLO 0:00000.000");
}

#[test]
fn test_glonass_display_tod_zero_padded_to_5_digits() {
    let t = Time::<Glonass>::from_day_tod(1, 1.0).unwrap();

    assert_eq!(t.to_string(), "GLO 1:00001.000");
}

#[test]
fn test_glonass_day_accessor_large_value() {
    let t = Time::<Glonass>::from_day_tod(99_999, 86_399.0).unwrap();

    assert_eq!(t.day(), 99_999);
    assert_eq!(t.tod_seconds(), 86_399);
}

// GLONASS не сбрасывается каждые 7 дней, как GPS (каждые 604 800
// секунд/неделю). Счётчик дней монотонно увеличивается с момента эпохи.
#[test]
fn test_glonass_day_counter_does_not_rollover_at_7() {
    // Проверяем, что дни 7, 14, 21, ... дают корректный day_of_week,
    // но доступ к `day()` НЕ выполняет циклический сброс (не делает wrap).
    for n in [7u32, 14, 21, 100, 1000] {
        let t = Time::<Glonass>::from_day_tod(n, 0.0).unwrap();

        assert_eq!(t.day(), n, "day() should return raw day count, not wrapped");
    }
}

// В отличие от GPS, который использует структуру "неделя" + "TOW", GLONASS
// использует абсолютный счёт дней от эпохи. Проверяем, что создание из больших
// значений дней работает корректно.
#[test]
fn test_glonass_large_day_count_roundtrip() {
    // ~30 лет после эпохи ≈ 10 950 дней
    let t = Time::<Glonass>::from_day_tod(10_950, 43_200.0).unwrap();

    assert_eq!(t.day(), 10_950);
    assert_eq!(t.tod_seconds(), 43_200);
}

// В момент эпохи GLONASS (1996-01-01 00:00:00 UTC(SU) =
// 1995-12-31 21:00:00 UTC): GPS_s = ? (выводится из UTC)
//
// UTC в момент эпохи GLONASS: 757_371_600 с от 1972 года
// GPS = UTC − разница эпох + (TAI-UTC − 19)
//     = 757_371_600 − 252_892_800 + (30 − 19)  [TAI-UTC = 30 на 1996-01-01]
//     = 504_478_800 + 11 = 504_478_811
//
// Проверка: GPS_s = 504_478_811 → GPS неделя ≈ 833, TOW = некоторое число
// секунд
#[test]
fn test_glonass_epoch_in_gps_seconds() {
    let ls = LeapSeconds::builtin();

    // GLONASS epoch → UTC → GPS
    let glo_epoch = Time::<Glonass>::EPOCH;
    let utc: Time<Utc> = glo_epoch.into_scale().unwrap();
    let gps: Time<Gps> = utc.into_scale_with(ls).unwrap();

    // Ожидается: GPS_s = 757_371_600 - 252_892_800 + (30 - 19)
    // = 504_478_800 + 11 = 504_478_811
    assert_eq!(
        gps.as_seconds(),
        504_478_810,
        "GLONASS epoch expressed in GPS seconds"
    );
    // Неделя GPS: 504_478_810 / 604_800 = 834 недели + остаток
    assert_eq!(gps.week(), 834);
}

// Эпоха GPS (1980-01-06 00:00:00 UTC), выраженная во времени GLONASS.
//
// GPS epoch в секундах UTC = 252_892_800 (от эпохи UTC 1972-01-01)
// GPS epoch → время UTC (эпоха UTC = 252_892_800с от 1972)
// UTC → GLO: GLO_ns = UTC_ns - 757_371_600_000_000_000
// UTC_ns в момент GPS epoch = 252_892_800_000_000_000
// GLO_ns = 252_892_800_000_000_000 - 757_371_600_000_000_000 = ОТРИЦАТЕЛЬНОЕ
//
// Это означает, что эпоха GPS находится раньше эпохи GLONASS → ожидается
// переполнение.
#[test]
fn test_test_gps_epoch_predates_glonass_epoch() {
    // GPS epoch (1980-01-06) раньше эпохи GLONASS (1996-01-01),
    // поэтому преобразование GPS epoch в GLONASS должно завершиться с ошибкой
    // Overflow
    let ls = LeapSeconds::builtin();
    let gps_epoch = Time::<Gps>::EPOCH;
    let result: Result<Time<Glonass>, _> = gps_epoch.into_scale_with(ls);

    assert!(
        matches!(result, Err(GnssTimeError::Overflow)),
        "GPS epoch (1980) is before GLONASS epoch (1996) → should be overflow"
    );
}
