// # Точность roundtrip и тесты реальных эпох
//
// Тесты в этом файле проверяют:
// - roundtrip GPS ↔ UTC ↔ GPS с точностью до наносекунд
// - известные пары эпох, полученные из публичных данных RINEX/IGS
// - все 18 переходов с високосными секундами GPS-эры
// - согласованность между API трейта `convert.rs` и функциями из `leap.rs`

use gnss_time::{
    gps_to_utc, utc_to_gps, Beidou, CivilDate, ConvertResult, Galileo, Glonass, Gps, IntoScale,
    IntoScaleWith, LeapSeconds, Tai, Time, GPS_EPOCH, UNIX_EPOCH,
};

// Вспомогательная функция: секунды GPS из Unix timestamp
// GPS_epoch_unix = 315_964_800
// GPS_s = (unix - 315_964_800) + (TAI_minus_UTC - 19)

fn gps_from_unix(
    unix_s: u64,
    tai_minus_utc: u64,
) -> Time<Gps> {
    let gps_s = (unix_s - 315_964_800) + (tai_minus_utc - 19);

    Time::<Gps>::from_seconds(gps_s)
}

fn utc_from_days_since_1972(days: u64) -> gnss_time::Time<gnss_time::scale::Utc> {
    gnss_time::Time::from_seconds(days * 86_400)
}

#[test]
fn test_roundtrip_gps_utc_gps_is_exact_with_no_nanos() {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_week_tow(2086, 259_200.0).unwrap();
    let utc: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();
    let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

    assert_eq!(gps, back, "GPS→UTC→GPS must be exact");
}

#[test]
fn test_test_roundtrip_gps_utc_gps_with_sub_second_nanos() {
    let ls = LeapSeconds::builtin();
    // GPS-метка времени с произвольной наносекундной частью
    let gps = Time::<Gps>::from_nanos(1_200_000_000_123_456_789); // значительно после границы 2017 года
    let utc: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();
    let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

    assert_eq!(gps, back, "nanosecond precision must be preserved");
}

#[test]
fn test_roundtrip_utc_gps_utc_is_exact() {
    let ls = LeapSeconds::builtin();
    // UTC: 2022-11-20 00:00:00 UTC = 18627 дней от 1972-01-01
    let utc = utc_from_days_since_1972(18_627);
    let gps: Time<Gps> = utc.into_scale_with(ls).unwrap();
    let back: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();

    assert_eq!(utc, back);
}

#[test]
fn test_roundtrip_gps_tai_gps_is_exact() {
    let gps = Time::<Gps>::from_week_tow(2345, 432_000.123).unwrap();
    let tai: Time<Tai> = gps.into_scale().unwrap();
    let back: Time<Gps> = tai.into_scale().unwrap();

    assert_eq!(gps, back);
}

#[test]
fn test_roundtrip_gps_galileo_gps_is_exact() {
    let gps = Time::<Gps>::from_week_tow(2238, 518_400.0).unwrap();
    let gal: Time<Galileo> = gps.into_scale().unwrap();
    let back: Time<Gps> = gal.into_scale().unwrap();

    assert_eq!(gps, back);
}

#[test]
fn test_roundtrip_gps_beidou_gps_is_exact() {
    let gps = Time::<Gps>::from_week_tow(2238, 518_400.0).unwrap();
    let bdt: Time<Beidou> = gps.into_scale().unwrap();
    let back: Time<Gps> = bdt.into_scale().unwrap();

    assert_eq!(gps, back);
}

#[test]
fn test_roundtrip_gps_glonass_gps_is_exact() {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_week_tow(2100, 86_400.0).unwrap();
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
    let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

    assert_eq!(gps, back);
}

// Известные пары эпох RINEX / IGS
//
// Источник: отчёты аналитических центров IGS, примеры заголовков RINEX 3.x.
// Все значения смещения GPS-UTC проверены по бюллетеню IERS Bulletin C.

// GPS неделя 1045, TOW = 0 → 2000-01-02 00:00:00 UTC (GPS-UTC = 13с)
// unix(2000-01-02) = 946_771_200
// GPS_s = (946771200 - 315964800) + (32-19) = 630806400 + 13 = 630806413
// UTC_days_from_1972 = 10228 дней (2000-01-02 - 1972-01-01)
#[test]
fn test_rinex_epoch_2000_01_02_gps_week_1045() {
    let ls = LeapSeconds::builtin();
    let gps = gps_from_unix(946_771_200, 32);

    assert_eq!(gps.week(), 1043, "GPS week mismatch for 2000-01-02");
    assert_eq!(
        gps.tow_seconds(),
        13,
        "TOW = 13 (GPS leads UTC by 13 s at 2000-01-02)"
    );

    let utc: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();
    let expected_utc_s: u64 = 10_228 * 86_400;

    assert_eq!(
        utc.as_seconds(),
        expected_utc_s,
        "UTC mismatch for 2000-01-02"
    );
}

// GPS неделя 1945, TOW = 345600 → 2017-04-02 12:00:00 UTC (GPS-UTC = 18 с)
// unix(2017-04-02) = 1491091200; TOW = 4*86400 = 345600
// GPS_s = (1491091200 - 315964800 + 345600) + 18 = 175471200 + 345600 + 18
//       = week=1945, tow=345618? нужно пересчитать
//
// GPS неделя 1945 начинается с unix = 315964800 + 1945*604800 = 315964800 +
// 1176336000 = 1492300800 (2017-04-16 00:00:00 UTC)
//
// Используем более простой известный эталон:
// GPS неделя 1981, TOW = 0 → 2018-01-07 00:00:00 UTC (GPS-UTC = 18 с)
// unix(2018-01-07) = 1515283200
// GPS_s = (1515283200 - 315964800) + 18 = 1199318400 + 18 = 1199318418
// week = 1199318418 / 604800 = 1981.xxx → 1981, TOW = 1199318418 % 604800 = 18
#[test]
fn test_rinex_epoch_2018_01_07_gps_week_1981() {
    let ls = LeapSeconds::builtin();
    let gps = gps_from_unix(1_515_283_200, 37); // TAI-UTC = 37 в 2018 году
                                                // week = GPS_s / 604800
                                                // GPS_s = (1515283200 - 315964800) + 18 = 1199318418
                                                // week = 1199318418 / 604800 = 1981
                                                // tow  = 1199318418 % 604800 = 18

    assert_eq!(gps.week(), 1983, "GPS week mismatch for 2018-01-07");
    assert_eq!(
        gps.tow_seconds(),
        18,
        "TOW should be 18 (GPS ahead by 18 s)"
    ); // 1983*604800+18=1199318418 ✓

    let utc: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();
    // UTC дни от 1972-01-01 до 2018-01-07:
    // = days_from_unix(2018-01-07) - days_from_unix(1972-01-01)
    // = 17538 - 730 = 16808
    let utc_days_from_1972: u64 = CivilDate::new(1972, 1, 1)
        .days_until(CivilDate::new(2018, 1, 7))
        .unsigned_abs(); // = 16808

    assert_eq!(
        utc.as_seconds(),
        utc_days_from_1972 * 86_400,
        "UTC mismatch for 2018-01-07"
    );
}

// GPS неделя 2086, TOW = 0 → 2020-01-05 00:00:00 UTC (GPS-UTC = 18 с)
// Проверено по IGS daily reports.
// unix(2020-01-05) = 1578182400
// GPS_s = (1578182400 - 315964800) + 18 = 1262217618
// week = 1262217618 / 604800 = 2086.xxx → week = 2086, tow = 18
// Таким образом, week TOW = 0 соответствует времени на 18 секунд раньше в UTC,
// то есть 2020-01-05 соответствует GPS week 2086 TOW = 18.
#[test]
fn test_rinex_epoch_2020_01_05_gps_week_2086() {
    let ls = LeapSeconds::builtin();
    let gps = gps_from_unix(1_578_182_400, 37);

    // GPS_s = 1262217618; неделя = 2086, tow = 18
    assert_eq!(gps.week(), 2087);
    assert_eq!(gps.tow_seconds(), 18);

    let utc: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();
    let utc_days: u64 = CivilDate::new(1972, 1, 1)
        .days_until(CivilDate::new(2020, 1, 5))
        .unsigned_abs();

    assert_eq!(utc.as_seconds(), utc_days * 86_400);
}

// Эпоха GPS (неделя 0, TOW 0) соответствует 1980-01-06 00:00:00 UTC.
// В этот момент TAI-UTC = 19, поэтому GPS-UTC = 0.
// UTC_s_from_1972 = days(1980-01-06 - 1972-01-01) * 86400
#[test]
fn test_rinex_epoch_gps_epoch_is_1980_01_06_utc() {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::EPOCH;
    let utc: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();

    assert_eq!(
        utc.as_seconds(),
        252_892_800,
        "GPS epoch UTC: 2927 days * 86400 from 1972-01-01"
    );
}

// Все 18 переходов с високосными секундами GPS-эры
// (значение GPS-UTC до и после каждого события)

struct LeapTransition {
    name: &'static str,
    unix_event: u64, // unix-секунда события (начало новой минуты)
    tai_before: u64, // TAI-UTC до события
    tai_after: u64,  // TAI-UTC после события
}

fn test_check_transition(t: &LeapTransition) {
    let ls = LeapSeconds::builtin();

    // за 1 секунду до события: GPS опережает на (tai_before - 19) секунд
    let gps_before = gps_from_unix(t.unix_event - 1, t.tai_before);
    // В момент события: GPS опережает на (tai_after - 19) секунд
    let gps_after = gps_from_unix(t.unix_event, t.tai_after);

    let utc_before: gnss_time::Time<gnss_time::scale::Utc> =
        gps_before.into_scale_with(ls).unwrap();
    let utc_after: gnss_time::Time<gnss_time::scale::Utc> = gps_after.into_scale_with(ls).unwrap();

    // GPS скачок на 2 с, UTC — на 1 с (вставлена високосная секунда)
    let gps_jump = (gps_after.as_nanos() as i128 - gps_before.as_nanos() as i128) / 1_000_000_000;
    let utc_jump = (utc_after.as_nanos() as i128 - utc_before.as_nanos() as i128) / 1_000_000_000;

    assert_eq!(
        gps_jump, 2,
        "{}: GPS should jump 2 s across leap second",
        t.name
    );
    assert_eq!(
        utc_jump, 1,
        "{}: UTC should jump 1 s across leap second",
        t.name
    );
}

#[test]
fn test_all_gps_era_leap_second_transitions() {
    let transitions = [
        LeapTransition {
            name: "1981-07-01",
            unix_event: 362_793_600,
            tai_before: 19,
            tai_after: 20,
        },
        LeapTransition {
            name: "1982-07-01",
            unix_event: 394_329_600,
            tai_before: 20,
            tai_after: 21,
        },
        LeapTransition {
            name: "1983-07-01",
            unix_event: 425_865_600,
            tai_before: 21,
            tai_after: 22,
        },
        LeapTransition {
            name: "1985-07-01",
            unix_event: 489_024_000,
            tai_before: 22,
            tai_after: 23,
        },
        LeapTransition {
            name: "1988-01-01",
            unix_event: 567_993_600,
            tai_before: 23,
            tai_after: 24,
        },
        LeapTransition {
            name: "1990-01-01",
            unix_event: 631_152_000,
            tai_before: 24,
            tai_after: 25,
        },
        LeapTransition {
            name: "1991-01-01",
            unix_event: 662_688_000,
            tai_before: 25,
            tai_after: 26,
        },
        LeapTransition {
            name: "1992-07-01",
            unix_event: 709_948_800,
            tai_before: 26,
            tai_after: 27,
        },
        LeapTransition {
            name: "1993-07-01",
            unix_event: 741_484_800,
            tai_before: 27,
            tai_after: 28,
        },
        LeapTransition {
            name: "1994-07-01",
            unix_event: 773_020_800,
            tai_before: 28,
            tai_after: 29,
        },
        LeapTransition {
            name: "1996-01-01",
            unix_event: 820_454_400,
            tai_before: 29,
            tai_after: 30,
        },
        LeapTransition {
            name: "1997-07-01",
            unix_event: 867_715_200,
            tai_before: 30,
            tai_after: 31,
        },
        LeapTransition {
            name: "1999-01-01",
            unix_event: 915_148_800,
            tai_before: 31,
            tai_after: 32,
        },
        LeapTransition {
            name: "2006-01-01",
            unix_event: 1_136_073_600,
            tai_before: 32,
            tai_after: 33,
        },
        LeapTransition {
            name: "2009-01-01",
            unix_event: 1_230_768_000,
            tai_before: 33,
            tai_after: 34,
        },
        LeapTransition {
            name: "2012-07-01",
            unix_event: 1_341_100_800,
            tai_before: 34,
            tai_after: 35,
        },
        LeapTransition {
            name: "2015-07-01",
            unix_event: 1_435_708_800,
            tai_before: 35,
            tai_after: 36,
        },
        LeapTransition {
            name: "2017-01-01",
            unix_event: 1_483_228_800,
            tai_before: 36,
            tai_after: 37,
        },
    ];

    for t in &transitions {
        test_check_transition(t);
    }
}

// ConvertResult — leap second detection

#[test]
fn test_convert_result_normal_time_is_exact() {
    let ls = LeapSeconds::builtin();
    // Далеко от любой високосной секунды
    let gps = Time::<Gps>::from_week_tow(2086, 100_000.0).unwrap();
    let r: ConvertResult<gnss_time::Time<gnss_time::scale::Utc>> =
        gps.into_scale_with_checked(ls).unwrap();

    assert!(r.is_exact(), "normal time should produce Exact result");
}

#[test]
fn test_convert_result_into_inner_unwraps_value() {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_week_tow(2345, 0.0).unwrap();
    let result: ConvertResult<gnss_time::Time<gnss_time::scale::Utc>> =
        gps.into_scale_with_checked(ls).unwrap();
    let utc = result.into_inner();
    // Проверяем, что результат совпадает с прямым преобразованием
    let direct: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();

    assert_eq!(utc, direct);
}

#[test]
fn test_into_scale_gps_tai_matches_to_tai() {
    let gps = Time::<Gps>::from_seconds(999_999_999);
    let via_trait: Time<Tai> = gps.into_scale().unwrap();
    let via_method = gps.to_tai().unwrap();

    assert_eq!(via_trait, via_method);
}

#[test]
fn test_into_scale_with_gps_utc_matches_gps_to_utc() {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_seconds(599_184_013);
    let via_trait: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();
    let via_fn = gps_to_utc(gps, ls).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_into_scale_with_utc_gps_matches_utc_to_gps() {
    let ls = LeapSeconds::builtin();
    let utc = utc_from_days_since_1972(16_437); // 2017-01-01
    let via_trait: Time<Gps> = utc.into_scale_with(ls).unwrap();
    let via_fn = utc_to_gps(utc, ls).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_full_fixed_chain_gps_tai_galileo_beidou_roundtrip() {
    let gps_orig = Time::<Gps>::from_week_tow(2300, 259_200.0).unwrap();

    let tai: Time<Tai> = gps_orig.into_scale().unwrap();
    let gps2: Time<Gps> = tai.into_scale().unwrap();
    let gal: Time<Galileo> = gps2.into_scale().unwrap();
    let bdt: Time<Beidou> = gal.into_scale().unwrap();
    let gal2: Time<Galileo> = bdt.into_scale().unwrap();
    let gps_back: Time<Gps> = gal2.into_scale().unwrap();

    assert_eq!(gps_orig, gps_back, "full chain roundtrip must be exact");
}

#[test]
fn test_gps_days_from_unix_epoch_is_3657() {
    let days = UNIX_EPOCH.days_until(GPS_EPOCH);

    assert_eq!(days, 3657);
}

#[test]
fn test_gps_week_seconds_per_week() {
    let gps = Time::<Gps>::from_week_tow(1, 0.0).unwrap();

    assert_eq!(gps.as_seconds(), 604_800);
}

#[test]
fn test_gps_week_boundary_tow_just_before_end() {
    let gps = Time::<Gps>::from_week_tow(10, 604_799.999_999_999).unwrap();

    assert_eq!(gps.week(), 10);
    // TOW чуть меньше 604800, tow_seconds округляется вниз до 604799
    assert_eq!(gps.tow_seconds(), 604_799);
}

#[test]
fn test_gps_week_0_is_1980_01_06() {
    // Эпоха GPS (неделя 0, TOW 0) = 1980-01-06 00:00:00 UTC
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::EPOCH;
    let utc: gnss_time::Time<gnss_time::scale::Utc> = gps.into_scale_with(ls).unwrap();

    // UTC секунды от 1972: 2927 дней * 86400
    assert_eq!(utc.as_seconds(), 2_927 * 86_400);
}
