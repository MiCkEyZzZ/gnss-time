use gnss_time::{
    epoch::{
        CivilDate, BEIDOU_EPOCH, DAYS_GPS_TO_BEIDOU, DAYS_GPS_TO_GALILEO, DAYS_GPS_TO_GLONASS,
        DAYS_UNIX_TO_GPS, GALILEO_EPOCH, GLONASS_EPOCH, GPS_EPOCH, LEAP_SECONDS_AT_BEIDOU_EPOCH,
        LEAP_SECONDS_AT_GLONASS_EPOCH, LEAP_SECONDS_AT_GPS_EPOCH,
        NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR, NANOS_GPS_TO_GALILEO_EPOCH, UNIX_EPOCH,
    },
    scale::{DisplayStyle, OffsetToTai, TimeScale},
    Beidou, Duration, DurationParts, Galileo, Glonass, GnssTimeError, Gps, Tai, Time, Utc,
};

#[test]
fn test_time_is_same_size_as_u64_for_all_scales() {
    assert_eq!(core::mem::size_of::<Time<Gps>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Glonass>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Galileo>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Beidou>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Tai>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Utc>>(), 8);
}

#[test]
fn test_duration_is_same_size_as_i64() {
    assert_eq!(core::mem::size_of::<Duration>(), 8);
}

#[test]
fn test_epoch_is_zero_nanos_for_every_scale() {
    assert_eq!(Time::<Gps>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Glonass>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Galileo>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Beidou>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Tai>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Utc>::EPOCH.as_nanos(), 0);
}

#[test]
fn test_gps_week_0_tow_0_is_epoch() {
    let t = Time::<Gps>::from_week_tow(
        0,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t, Time::<Gps>::EPOCH);
}

#[test]
fn test_gps_week_tow_roundtrip() {
    let t = Time::<Gps>::from_week_tow(
        2300,
        DurationParts {
            seconds: 12_345,
            nanos: 678_000_000,
        },
    )
    .unwrap();

    assert_eq!(t.week(), 2300);
    assert_eq!(t.tow_seconds(), 12_345);
    assert_eq!(t.sub_second_nanos(), 678_000_000);
}

#[test]
fn test_gps_tow_boundary_valid() {
    assert!(Time::<Gps>::from_week_tow(
        0,
        DurationParts {
            seconds: 604_799,
            nanos: 999_999_999
        }
    )
    .is_ok());
}

#[test]
fn test_gps_tow_at_604800_is_invalid() {
    assert!(matches!(
        Time::<Gps>::from_week_tow(
            0,
            DurationParts {
                seconds: 604_800,
                nanos: 0
            }
        ),
        Err(GnssTimeError::InvalidInput(_))
    ));
}

#[test]
fn test_gps_invalid_tow_bounds() {
    // >= 1 week
    assert!(matches!(
        Time::<Gps>::from_week_tow(
            0,
            DurationParts {
                seconds: 604_800,
                nanos: 0
            }
        ),
        Err(GnssTimeError::InvalidInput(_))
    ));
}

#[test]
fn test_gps_invalid_nanos() {
    assert!(matches!(
        Time::<Gps>::from_week_tow(
            0,
            DurationParts {
                seconds: 0,
                nanos: 1_000_000_000
            }
        ),
        Err(GnssTimeError::InvalidInput(_))
    ));
}

#[test]
fn test_glonass_day_0_tod_0_is_epoch() {
    let t = Time::<Glonass>::from_day_tod(
        0,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t, Time::<Glonass>::EPOCH);
}

#[test]
fn test_glonass_tod_boundary_valid() {
    assert!(Time::<Glonass>::from_day_tod(
        100,
        DurationParts {
            seconds: 86_399,
            nanos: 999_999_999
        }
    )
    .is_ok());
}

#[test]
fn test_glonass_tod_at_86400_is_invalid() {
    assert!(matches!(
        Time::<Glonass>::from_day_tod(
            0,
            DurationParts {
                seconds: 86_400,
                nanos: 0
            }
        ),
        Err(GnssTimeError::InvalidInput(_))
    ));
}

#[test]
fn test_glonass_day_and_tod_accessors() {
    let t = Time::<Glonass>::from_day_tod(
        10_512,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.day(), 10_512);
    assert_eq!(t.tod_seconds(), 43_200);
}

#[test]
fn test_duration_unit_constructors_are_consistent() {
    assert_eq!(Duration::from_days(1), Duration::from_hours(24));
    assert_eq!(Duration::from_hours(1), Duration::from_minutes(60));
    assert_eq!(Duration::from_minutes(1), Duration::from_seconds(60));
    assert_eq!(Duration::from_seconds(1), Duration::from_millis(1_000));
    assert_eq!(Duration::from_millis(1), Duration::from_micros(1_000));
    assert_eq!(Duration::from_micros(1), Duration::from_nanos(1_000));
}

#[test]
fn test_adding_one_week_to_epoch() {
    let one_week_ns = 604_800u64 * 1_000_000_000u64;
    let t = Time::<Gps>::EPOCH + Duration::from_seconds(604_800);

    assert_eq!(t.as_nanos(), one_week_ns);
}

#[test]
fn test_subtracting_duration_is_inverse_of_adding() {
    let base = Time::<Beidou>::from_seconds(1_000_000);
    let d = Duration::from_seconds(42_000);

    assert_eq!((base + d) - d, base);
}

#[test]
fn test_elapsed_between_two_gps_timestamps() {
    let start = Time::<Gps>::from_week_tow(
        2300,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let end = Time::<Gps>::from_week_tow(
        2300,
        DurationParts {
            seconds: 3600,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!((end - start).as_seconds(), 3600);
}

#[test]
fn test_negative_elapsed() {
    let a = Time::<Tai>::from_seconds(100);
    let b = Time::<Tai>::from_seconds(200);

    assert_eq!((a - b).as_seconds(), -100);
}

#[test]
fn test_checked_add_overflow() {
    assert!(Time::<Gps>::MAX
        .checked_add(Duration::ONE_NANOSECOND)
        .is_none());
}

#[test]
fn test_saturating_add_clamps_to_max() {
    assert_eq!(
        Time::<Gps>::MAX.saturating_add(Duration::from_seconds(999)),
        Time::<Gps>::MAX
    );
}

#[test]
fn test_try_add_returns_err_on_overflow() {
    assert!(matches!(
        Time::<Gps>::MAX.try_add(Duration::ONE_NANOSECOND),
        Err(GnssTimeError::Overflow)
    ));
}

#[test]
fn test_timestamps_sort_correctly() {
    let t0 = Time::<Gps>::from_seconds(0);
    let t1 = Time::<Gps>::from_seconds(1);
    let t2 = Time::<Gps>::from_seconds(2);
    let mut v = vec![t2, t0, t1];

    v.sort();

    assert_eq!(v, vec![t0, t1, t2]);
}

#[test]
fn test_gps_display_canonical_example() {
    // Exact format from the issue: "GPS 2345:432000.000"
    let t = Time::<Gps>::from_week_tow(
        2345,
        DurationParts {
            seconds: 432_000,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.to_string(), "GPS 2345:432000.000");
}

#[test]
fn test_gps_display_epoch_is_zero_week() {
    assert_eq!(Time::<Gps>::EPOCH.to_string(), "GPS 0:000000.000");
}

#[test]
fn test_gps_display_tow_is_zero_padded_to_6_digits() {
    let t = Time::<Gps>::from_week_tow(
        10,
        DurationParts {
            seconds: 1,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.to_string(), "GPS 10:000001.000");
}

#[test]
fn test_gps_display_millis_precision() {
    let t = Time::<Gps>::from_week_tow(
        0,
        DurationParts {
            seconds: 0,
            nanos: 123_000_000,
        },
    )
    .unwrap();

    assert_eq!(t.to_string(), "GPS 0:000000.123");
}

#[test]
fn test_glonass_display_canonical() {
    let t = Time::<Glonass>::from_day_tod(
        10_512,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.to_string(), "GLO 10512:43200.000");
}

#[test]
fn test_glonass_display_epoch() {
    assert_eq!(Time::<Glonass>::EPOCH.to_string(), "GLO 0:00000.000");
}

#[test]
fn test_glonass_display_tod_is_zero_padded_to_5_digits() {
    let t = Time::<Glonass>::from_day_tod(
        5,
        DurationParts {
            seconds: 1,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.to_string(), "GLO 5:00001.000");
}

#[test]
fn test_galileo_display_uses_week_tow_format() {
    let s = Time::<Galileo>::EPOCH.to_string();

    assert!(s.starts_with("GAL "));
    assert!(s.contains(':'));
    assert!(s.contains('.'));
}

#[test]
fn test_beidou_display_uses_week_tow_format() {
    let s = Time::<Beidou>::EPOCH.to_string();

    assert!(s.starts_with("BDT "));
}

#[test]
fn test_tai_display_uses_simple_format() {
    let s = Time::<Tai>::from_seconds(1_000_000_000).to_string();

    assert_eq!(s, "TAI +1000000000s 0ns");
}

#[test]
fn test_utc_display_uses_simple_format() {
    assert!(Time::<Utc>::EPOCH.to_string().starts_with("UTC +"));
}

#[test]
fn test_display_styles_match_expected() {
    assert_eq!(Gps::DISPLAY_STYLE, DisplayStyle::WeekTow);
    assert_eq!(Glonass::DISPLAY_STYLE, DisplayStyle::DayTod);
    assert_eq!(Galileo::DISPLAY_STYLE, DisplayStyle::WeekTow);
    assert_eq!(Beidou::DISPLAY_STYLE, DisplayStyle::WeekTow);
    assert_eq!(Tai::DISPLAY_STYLE, DisplayStyle::Simple);
    assert_eq!(Utc::DISPLAY_STYLE, DisplayStyle::Simple);
}

#[test]
fn test_epoch_civil_dates_are_correct() {
    assert_eq!(Gps::EPOCH_CIVIL.year, 1980);
    assert_eq!(Gps::EPOCH_CIVIL.month, 1);
    assert_eq!(Gps::EPOCH_CIVIL.day, 6);

    assert_eq!(Glonass::EPOCH_CIVIL.year, 1996);
    assert_eq!(Glonass::EPOCH_CIVIL.month, 1);
    assert_eq!(Glonass::EPOCH_CIVIL.day, 1);

    assert_eq!(Galileo::EPOCH_CIVIL.year, 1999);
    assert_eq!(Galileo::EPOCH_CIVIL.month, 8);
    assert_eq!(Galileo::EPOCH_CIVIL.day, 22);

    assert_eq!(Beidou::EPOCH_CIVIL.year, 2006);
    assert_eq!(Beidou::EPOCH_CIVIL.month, 1);
    assert_eq!(Beidou::EPOCH_CIVIL.day, 1);
}

#[test]
fn test_gps_epoch_is_3657_days_from_unix() {
    assert_eq!(DAYS_UNIX_TO_GPS, 3657);
    assert_eq!(GPS_EPOCH.days_from_unix(), 3657);
}

#[test]
fn test_gps_to_galileo_delta_matches_igs_standard() {
    assert_eq!(DAYS_GPS_TO_GALILEO, 7168);
    assert_eq!(GPS_EPOCH.seconds_until(GALILEO_EPOCH), 619_315_200);
    assert_eq!(NANOS_GPS_TO_GALILEO_EPOCH, 619_315_200_000_000_000_i64);
}

#[test]
fn test_gps_to_beidou_delta_matches_igs_standard() {
    assert_eq!(DAYS_GPS_TO_BEIDOU, 9492);
    assert_eq!(GPS_EPOCH.seconds_until(BEIDOU_EPOCH), 820_108_800);
    assert_eq!(
        NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR,
        820_108_800_000_000_000_i64
    );
}

#[test]
fn test_gps_to_glonass_delta() {
    assert_eq!(DAYS_GPS_TO_GLONASS, 5839);
    assert_eq!(GPS_EPOCH.seconds_until(GLONASS_EPOCH), 5839 * 86_400);
}

#[test]
fn test_leap_seconds_at_epochs_are_correct() {
    assert_eq!(LEAP_SECONDS_AT_GPS_EPOCH, 19);
    assert_eq!(LEAP_SECONDS_AT_GLONASS_EPOCH, 30);
    assert_eq!(LEAP_SECONDS_AT_BEIDOU_EPOCH, 33);
}

#[test]
fn test_civil_date_arithmetic_self_distance_is_zero() {
    assert_eq!(GPS_EPOCH.days_until(GPS_EPOCH), 0);
    assert_eq!(UNIX_EPOCH.days_until(UNIX_EPOCH), 0);
}

#[test]
fn test_civil_date_arithmetic_is_antisymmetric() {
    let a = CivilDate::new(2000, 1, 1);
    let b = CivilDate::new(2001, 6, 15);

    assert_eq!(a.days_until(b), -b.days_until(a));
}

#[test]
fn test_gps_to_tai_adds_19s() {
    let gps = Time::<Gps>::from_seconds(100);
    let tai = gps.to_tai().unwrap();

    assert_eq!(tai.as_seconds(), 119);
}

#[test]
fn test_tai_to_gps_subtracts_19s() {
    let tai = Time::<Tai>::from_seconds(119);
    let gps = Time::<Gps>::from_tai(tai).unwrap();

    assert_eq!(gps.as_seconds(), 100);
}

#[test]
fn test_gps_galileo_roundtrip_via_tai_is_identity() {
    // GPS and Galileo use the same TAI offset -> identical nanoseconds
    let gps = Time::<Gps>::from_seconds(12_345);
    let gal = gps.try_convert::<Galileo>().unwrap();

    assert_eq!(gps.as_nanos(), gal.as_nanos());
}

#[test]
fn test_gps_to_beidou_via_tai() {
    // GPS(100 s) -> TAI(119 s) -> BDT(119-33 = 86 s)
    let gps = Time::<Gps>::from_seconds(100);
    let bdt = gps.try_convert::<Beidou>().unwrap();

    assert_eq!(bdt.as_seconds(), 86);
}

#[test]
fn test_glonass_to_tai_requires_context() {
    assert!(matches!(
        Time::<Glonass>::from_seconds(100).to_tai(),
        Err(GnssTimeError::LeapSecondsRequired)
    ));
}

#[test]
fn test_utc_to_tai_requires_context() {
    assert!(matches!(
        Time::<Utc>::from_seconds(100).to_tai(),
        Err(GnssTimeError::LeapSecondsRequired)
    ));
}

#[test]
fn test_tai_underflow_is_detected() {
    // TAI(0) - GPS offset (19 s) -> negative value -> Overflow
    let tai = Time::<Tai>::from_nanos(0);

    assert!(matches!(
        Time::<Gps>::from_tai(tai),
        Err(GnssTimeError::Overflow)
    ));
}

#[test]
fn test_beidou_roundtrip_via_tai() {
    let t = Time::<Beidou>::from_seconds(1_000_000);
    let back = Time::<Beidou>::from_tai(t.to_tai().unwrap()).unwrap();

    assert_eq!(t, back);
}

#[test]
fn test_offset_to_tai_constants() {
    const NS: i64 = 1_000_000_000;

    assert_eq!(Gps::OFFSET_TO_TAI, OffsetToTai::Fixed(19 * NS));
    assert_eq!(Galileo::OFFSET_TO_TAI, OffsetToTai::Fixed(19 * NS));
    assert_eq!(Beidou::OFFSET_TO_TAI, OffsetToTai::Fixed(33 * NS));
    assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
    assert!(Glonass::OFFSET_TO_TAI.is_contextual());
    assert!(Utc::OFFSET_TO_TAI.is_contextual());
}
