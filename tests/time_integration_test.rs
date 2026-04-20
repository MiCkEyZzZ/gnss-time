use gnss_time::{Beidou, Duration, Galileo, Glonass, GnssTimeError, Gps, Tai, Time, Utc};

#[test]
fn time_is_same_size_as_u64_for_all_scales() {
    assert_eq!(core::mem::size_of::<Time<Gps>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Glonass>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Galileo>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Beidou>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Tai>>(), 8);
    assert_eq!(core::mem::size_of::<Time<Utc>>(), 8);
}

#[test]
fn duration_is_same_size_as_i64() {
    assert_eq!(core::mem::size_of::<Duration>(), 8);
}

#[test]
fn epoch_is_zero_nanos_for_every_scale() {
    assert_eq!(Time::<Gps>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Glonass>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Galileo>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Beidou>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Tai>::EPOCH.as_nanos(), 0);
    assert_eq!(Time::<Utc>::EPOCH.as_nanos(), 0);
}

#[test]
fn gps_week_0_tow_0_is_epoch() {
    let t = Time::<Gps>::from_week_tow(0, 0.0).unwrap();
    assert_eq!(t, Time::<Gps>::EPOCH);
}

#[test]
fn gps_week_tow_roundtrip() {
    // GPS week 2300, TOW = 12345.678 s
    let t = Time::<Gps>::from_week_tow(2300, 12_345.678).unwrap();
    assert_eq!(t.week(), 2300);
    assert_eq!(t.tow_seconds(), 12_345);
    // sub-second part: 0.678 s = 678_000_000 ns
    assert_eq!(t.sub_second_nanos(), 678_000_000);
}

#[test]
fn gps_tow_boundary_valid() {
    // Last valid TOW: just under 604_800 s
    assert!(Time::<Gps>::from_week_tow(0, 604_799.999_999).is_ok());
}

#[test]
fn gps_tow_at_604800_is_invalid() {
    assert!(matches!(
        Time::<Gps>::from_week_tow(0, 604_800.0),
        Err(GnssTimeError::InvalidInput(_))
    ));
}

#[test]
fn gps_negative_tow_is_invalid() {
    assert!(matches!(
        Time::<Gps>::from_week_tow(0, -0.001),
        Err(GnssTimeError::InvalidInput(_))
    ));
}

#[test]
fn glonass_day_0_tod_0_is_epoch() {
    let t = Time::<Glonass>::from_day_tod(0, 0.0).unwrap();
    assert_eq!(t, Time::<Glonass>::EPOCH);
}

#[test]
fn glonass_tod_boundary_valid() {
    assert!(Time::<Glonass>::from_day_tod(100, 86_399.999_9).is_ok());
}

#[test]
fn glonass_tod_at_86400_is_invalid() {
    assert!(matches!(
        Time::<Glonass>::from_day_tod(0, 86_400.0),
        Err(GnssTimeError::InvalidInput(_))
    ));
}

#[test]
fn duration_unit_constructors_are_consistent() {
    assert_eq!(Duration::from_days(1), Duration::from_hours(24));
    assert_eq!(Duration::from_hours(1), Duration::from_minutes(60));
    assert_eq!(Duration::from_minutes(1), Duration::from_seconds(60));
    assert_eq!(Duration::from_seconds(1), Duration::from_millis(1_000));
    assert_eq!(Duration::from_millis(1), Duration::from_micros(1_000));
    assert_eq!(Duration::from_micros(1), Duration::from_nanos(1_000));
}

#[test]
fn adding_one_week_to_epoch_equals_one_week_nanos() {
    let one_week_ns = 604_800u64 * 1_000_000_000u64;
    let t = Time::<Gps>::EPOCH + Duration::from_seconds(604_800);
    assert_eq!(t.as_nanos(), one_week_ns);
}

#[test]
fn adding_negative_duration_subtracts() {
    let t = Time::<Gps>::from_seconds(100);
    let result = t + Duration::from_nanos(-10_000_000_000); // -10 s
    assert_eq!(result.as_seconds(), 90);
}

#[test]
fn add_assign_works() {
    let mut t = Time::<Galileo>::from_seconds(0);
    t += Duration::from_seconds(3600);
    assert_eq!(t.as_seconds(), 3600);
}

#[test]
fn subtracting_duration_is_inverse_of_adding() {
    let base = Time::<Beidou>::from_seconds(1_000_000);
    let d = Duration::from_seconds(42_000);
    assert_eq!((base + d) - d, base);
}

#[test]
fn elapsed_between_two_gps_timestamps() {
    let start = Time::<Gps>::from_week_tow(2300, 0.0).unwrap();
    let end = Time::<Gps>::from_week_tow(2300, 3600.0).unwrap();
    let elapsed = end - start;
    assert_eq!(elapsed.as_seconds(), 3600);
    assert!(elapsed.is_positive());
}

#[test]
fn negative_elapsed_when_subtracted_in_wrong_order() {
    let a = Time::<Tai>::from_seconds(100);
    let b = Time::<Tai>::from_seconds(200);
    assert_eq!((a - b).as_seconds(), -100);
}

#[test]
fn subtracting_equal_timestamps_gives_zero_duration() {
    let t = Time::<Utc>::from_nanos(123_456_789);
    assert!((t - t).is_zero());
}

#[test]
fn checked_add_overflow() {
    assert!(Time::<Gps>::MAX
        .checked_add(Duration::ONE_NANOSECOND)
        .is_none());
}

#[test]
fn checked_sub_duration_underflow_at_epoch() {
    let t = Time::<Gps>::EPOCH;
    assert!(t.checked_sub_duration(Duration::ONE_NANOSECOND).is_none());
}

#[test]
fn saturating_add_clamps_to_max() {
    assert_eq!(
        Time::<Gps>::MAX.saturating_add(Duration::from_seconds(999)),
        Time::<Gps>::MAX
    );
}

#[test]
fn saturating_sub_duration_clamps_to_epoch() {
    assert_eq!(
        Time::<Gps>::EPOCH.saturating_sub_duration(Duration::ONE_NANOSECOND),
        Time::<Gps>::EPOCH
    );
}

#[test]
fn try_add_returns_err_on_overflow() {
    assert!(matches!(
        Time::<Gps>::MAX.try_add(Duration::ONE_NANOSECOND),
        Err(GnssTimeError::Overflow)
    ));
}

#[test]
fn timestamps_order_correctly() {
    let t0 = Time::<Gps>::from_seconds(0);
    let t1 = Time::<Gps>::from_seconds(1);
    let t2 = Time::<Gps>::from_seconds(2);
    let mut v = vec![t2, t0, t1];
    v.sort();
    assert_eq!(v, vec![t0, t1, t2]);
}

#[test]
fn display_gps_epoch_is_sensible() {
    let s = Time::<Gps>::EPOCH.to_string();
    assert!(s.contains("GPS"));
    assert!(s.contains("+0s"));
}

#[test]
fn debug_shows_scale_name_and_nanos() {
    let t = Time::<Glonass>::from_nanos(777);
    let dbg = format!("{t:?}");
    assert!(dbg.contains("GLO"));
    assert!(dbg.contains("777"));
}

#[test]
fn duration_display_roundtrip() {
    let d = Duration::from_nanos(-3_141_592_654);
    let s = d.to_string();
    assert!(s.starts_with('-'));
    assert!(s.contains('s'));
}
