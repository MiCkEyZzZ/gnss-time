// Tests for task GLONASS transformations
//
// Why leap seconds are not needed for GLONASS ↔ UTC conversion
//
// GLONASS transmits time in UTC(SU) — Moscow standard time = UTC + 3 hours.
// Importantly, GLONASS **accounts for leap second insertions in the same way as
// UTC**: when IERS adds a leap second to UTC, GLONASS adds the same second to
// its own time scale.
//
// This means that both UTC and GLONASS continuously count nanoseconds in sync
// (in step with each other) — they differ only by a fixed epoch offset
// (the calendar difference between 1972-01-01 and the GLONASS epoch
// 1995-12-31 21:00:00 UTC).
// Therefore, leap seconds do not need to be considered when converting between
// them.
//
// However, leap seconds are needed for GLONASS ↔ GPS conversion, because GPS
// does not contain leap seconds (its time scale diverges from UTC by the
// accumulated number of leap seconds since 1980).
//
// Epoch geometry
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
    glonass_to_gps, glonass_to_utc, gps_to_glonass, utc_to_glonass, CivilDate, DurationParts,
    Glonass, GnssTimeError, Gps, IntoScale, IntoScaleWith, LeapSeconds, Time, Utc,
};

// GLONASS epoch = 1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC.
// In UTC nanoseconds from 1972-01-01: 8766 days × 86 400 s − 3 × 3 600 s
// = 757 382 400 − 10 800 = 757 371 600 s = 757_371_600_000_000_000 ns.
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
    // 1996-01-01 was a Monday → day_of_week = 1
    let t = Time::<Glonass>::EPOCH;

    assert_eq!(t.day(), 0);
    assert_eq!(t.day_of_week(), 1); // Monday
}

#[test]
fn test_glonass_utc_offset_is_exactly_3_hours() {
    // UTC(SU) = UTC + 3 hours → GLONASS is 3 hours ahead of UTC in clock time.
    //
    // In our representation, GLONASS epoch = UTC epoch + 757_371_600 seconds
    // (≈ 8766 days − 3 hours).
    // A GLONASS timestamp T corresponds to UTC timestamp:
    // (T + 757_371_600_000_000_000).
    //
    // Another way to understand this:
    // at GLONASS midnight (tod = 0), UTC time is 21:00 the previous day.
    //
    // Check for a known date:
    // GLONASS day 1, tod = 0 = 1996-01-02 00:00:00 UTC(SU)
    //                         = 1996-01-01 21:00:00 UTC
    let glo = Time::<Glonass>::from_day_tod(
        1,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let utc: Time<Utc> = glo.into_scale().unwrap();

    // UTC time 1996-01-01 21:00:00 can be obtained as:
    // GLONASS epoch (in nanoseconds relative to UTC) plus 1 day (86400 seconds)
    let expected_utc_s = 757_371_600_u64 + 86_400;

    assert_eq!(utc.as_seconds(), expected_utc_s);
}

#[test]
fn test_glonass_to_utc_is_constant_shift() {
    // The offset is always the same regardless of the time instant — no table
    // lookup is required.
    let glo1 = Time::<Glonass>::from_day_tod(
        1_000,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();
    let glo2 = Time::<Glonass>::from_day_tod(
        5_000,
        DurationParts {
            seconds: 12_345,
            nanos: 0,
        },
    )
    .unwrap();

    let utc1: Time<Utc> = glo1.into_scale().unwrap();
    let utc2: Time<Utc> = glo2.into_scale().unwrap();

    // The difference between UTC timestamps must equal the difference between GLO
    // timestamps
    let glo_diff = (glo2 - glo1).as_nanos();
    let utc_diff = (utc2 - utc1).as_nanos();

    assert_eq!(
        glo_diff, utc_diff,
        "GLONASS -> UTC is a rigid shift: intervals must be preserved"
    );
}

#[test]
fn test_glonass_to_utc_roundtrip() {
    let glo = Time::<Glonass>::from_day_tod(
        10_512,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();
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
    // UTC epoch (t = 0) precedes the GLONASS epoch -> underflow
    let utc = Time::<Utc>::EPOCH;
    let result: Result<Time<Glonass>, _> = utc.into_scale();

    assert!(matches!(result, Err(GnssTimeError::Overflow)));
}

#[test]
fn test_utc_just_at_glonass_epoch_gives_zero() {
    // UTC at 757_371_600 s from the UTC epoch = GLONASS epoch
    let utc = Time::<Utc>::from_nanos(757_371_600_000_000_000);
    let glo: Time<Glonass> = utc.into_scale().unwrap();

    assert_eq!(glo, Time::<Glonass>::EPOCH);
}

// At midnight in GLONASS time, UTC shows 21:00:00 (= UTC+3 offset).
// Check for a specific known date: 1996-01-01 00:00:00 UTC(SU).
#[test]
fn test_glonass_midnight_is_utc_21h() {
    // GLO day 0 tod 0 = 1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC
    let glo = Time::<Glonass>::EPOCH; // день 0, тод 0
    let utc: Time<Utc> = glo.into_scale().unwrap();

    // UTC: 757_371_600 s since 1972-01-01
    // 757_371_600 / 86400 = 8765 days + 21 hours
    let secs = utc.as_seconds();
    let hours_in_day = (secs % 86_400) / 3600;

    assert_eq!(hours_in_day, 21, "GLONASS midnight -> 21:00 UTC");
}

#[test]
fn test_glonass_gps_roundtrip_post_2017() {
    let ls = LeapSeconds::builtin();

    // After 2017-01-01 (the last leap second), GPS-UTC = 18 s (stable time)
    let gps = Time::<Gps>::from_week_tow(
        2100,
        DurationParts {
            seconds: 86_400,
            nanos: 0,
        },
    )
    .unwrap();
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
    let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

    assert_eq!(gps, back);
}

#[test]
fn test_glonass_gps_roundtrip_before_1999() {
    let ls = LeapSeconds::builtin();

    // Before 1999-01-01, GPS-UTC = 13 s
    // GPS_s = 504_478_810+ — this is after the GLONASS epoch, use 550_000_000
    // (~June 1997)
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

// Compare GLONASS and GPS at the same UTC instant.
//
// At 2020-05-01 00:00:18 GPS (= 2020-05-01 00:00:00 UTC, GPS-UTC=18):
// - GPS seconds: 1262217618
// - The same UTC instant in GLONASS should be 2020-05-01 03:00:00 UTC(SU) = day
//   8770 from the GLONASS epoch (1996-01-01) with tod = 10800 s (3 h)
#[test]
fn test_glonass_and_gps_at_same_utc_instant() {
    let ls = LeapSeconds::builtin();

    // GPS time on 2020-05-01 00:00:00 UTC:
    // unix = 1578182400, GPS_s = (1578182400 - 315964800) + 18 = 1262217618
    let gps = Time::<Gps>::from_seconds(1_262_217_618);
    let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();

    // 2020-05-01 03:00:00 UTC (SU) relative to the GLONASS epoch:
    // days from 01.01.1996 to 05.01.2020 = 8766 + 4 = 8770 days (verified below)
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

// GLONASS epoch = 1996-01-01 = Monday → day_of_week() = 1.
#[test]
fn test_day_of_week_epoch_is_monday() {
    let t = Time::<Glonass>::EPOCH;

    assert_eq!(t.day_of_week(), 1, "1996-01-01 was a Monday -> 1");
}

#[test]
fn test_day_of_week_sequence_mon_through_sun() {
    let expected = [1u8, 2, 3, 4, 5, 6, 7]; // Mon … Sun
    for (i, &expected_dow) in expected.iter().enumerate() {
        let t = Time::<Glonass>::from_day_tod(
            i as u32,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();
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
    // Day 7 = 1996-01-08 = Monday again
    let t = Time::<Glonass>::from_day_tod(
        7,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.day_of_week(), 1, "day 7 should be Monday again");
}

#[test]
fn test_day_of_week_saturday_and_sunday() {
    let sat = Time::<Glonass>::from_day_tod(
        5,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap(); // 1996-01-06 Saturday
    let sun = Time::<Glonass>::from_day_tod(
        6,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap(); // 1996-01-07 Sanday
    let mon = Time::<Glonass>::from_day_tod(
        0,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap(); // Monday

    assert_eq!(sat.day_of_week(), 6);
    assert_eq!(sun.day_of_week(), 7);
    assert_eq!(mon.day_of_week(), 1);
}

#[test]
fn test_is_weekend_returns_true_for_sat_sun() {
    let sat = Time::<Glonass>::from_day_tod(
        5,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let sun = Time::<Glonass>::from_day_tod(
        6,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let fri = Time::<Glonass>::from_day_tod(
        4,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let mon = Time::<Glonass>::from_day_tod(
        0,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert!(sat.is_weekend());
    assert!(sun.is_weekend());
    assert!(!fri.is_weekend());
    assert!(!mon.is_weekend());
}

// Check day_of_week for a known date: 2020-01-05 was Sunday.
// Number of days from 1996-01-01 to 2020-01-05 = 8770.
// 8770 % 7 = 1 → day_of_week = 2 (Tuesday)?
// Wait, I will recheck with Python...
// Actually: 8770 % 7 = 8770 mod 7.
// 8770 / 7 = 1252.857... → 1252 * 7 = 8764 → 8770 - 8764 = 6 → (6 % 7) + 1 = 7
// (Sunday) ✓
#[test]
fn test_day_of_week_2020_01_05_is_sunday() {
    let days = CivilDate::new(1996, 1, 1)
        .days_until(CivilDate::new(2020, 1, 5))
        .unsigned_abs() as u32;

    assert_eq!(days, 8770);

    let t = Time::<Glonass>::from_day_tod(
        days,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.day_of_week(), 7, "2020-01-05 was a Sunday");
}

#[test]
fn test_day_of_week_matches_known_dates() {
    // Verified by calendar: 2023-10-09 = Monday, number of days from
    // 1996-01-01 to 2023-10-09:
    let days_mon = CivilDate::new(1996, 1, 1)
        .days_until(CivilDate::new(2023, 10, 9))
        .unsigned_abs() as u32;
    let t = Time::<Glonass>::from_day_tod(
        days_mon,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.day_of_week(), 1, "2023-10-09 should be Monday");

    // 2023-10-14 = Saturday
    let days_sat = CivilDate::new(1996, 1, 1)
        .days_until(CivilDate::new(2023, 10, 14))
        .unsigned_abs() as u32;
    let t = Time::<Glonass>::from_day_tod(
        days_sat,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.day_of_week(), 6, "2023-10-14 should be Saturday");
}

#[test]
fn test_glonass_sub_second_nanos() {
    let t = Time::<Glonass>::from_day_tod(
        100,
        DurationParts {
            seconds: 43_200,
            nanos: 500_000_000,
        },
    )
    .unwrap();

    assert_eq!(t.tod_seconds(), 43_200);
    assert_eq!(t.sub_second_nanos(), 500_000_000); // 0.5c
}

#[test]
fn test_glonass_sub_second_nanos_zero() {
    let t = Time::<Glonass>::from_day_tod(
        0,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.sub_second_nanos(), 0);
}

// During the leap second in GPS (GPS “jumps” by 2 s, UTC increases by 1 s):
// the corresponding GLONASS timestamps also reflect the same continuity of UTC.
// Converting GPS -> GLO -> back must produce an exact roundtrip on both sides
// of this event.
#[test]
fn test_glonass_across_2017_leap_second_roundtrip() {
    let ls = LeapSeconds::builtin();

    // GPS before the 2017-01-01 leap second (well before the boundary): GPS_s =
    // 1167264010
    let gps_before = Time::<Gps>::from_seconds(1_167_264_010);
    // GPS after (well after the boundary): GPS_s = 1167264025
    let gps_after = Time::<Gps>::from_seconds(1_167_264_025);

    let glo_before: Time<Glonass> = gps_before.into_scale_with(ls).unwrap();
    let glo_after: Time<Glonass> = gps_after.into_scale_with(ls).unwrap();

    // Check roundtrip in both directions
    let back_before: Time<Gps> = glo_before.into_scale_with(ls).unwrap();
    let back_after: Time<Gps> = glo_after.into_scale_with(ls).unwrap();

    assert_eq!(gps_before, back_before);
    assert_eq!(gps_after, back_after);

    // UTC interval across the leap second: GLONASS advances by 2 s (as does
    // GPS) because GLONASS and UTC use the same leap second — both “jump”
    // together. GPS made a 15 s jump between our test points; GLONASS should give
    // the same jump (both synchronously follow UTC, including the insertion of
    // the 1-second leap second)
    let glo_jump_ns = glo_after.as_nanos() as i128 - glo_before.as_nanos() as i128;
    let gps_jump_ns = gps_after.as_nanos() as i128 - gps_before.as_nanos() as i128;

    // GPS jumped by 15 s, but UTC (and GLONASS) by only 14 s, because
    // 1 of these GPS seconds was “absorbed” by the leap second insertion.
    // This is correct: the GPS−UTC difference increased by 1 s across the boundary.
    assert_eq!(
        glo_jump_ns / 1_000_000_000,
        gps_jump_ns / 1_000_000_000 - 1,
        "GLONASS jumps 1 s less than GPS across a leap second (leap consumed 1 s)"
    );
}

#[test]
fn test_into_scale_glonass_utc_matches_glonass_to_utc() {
    let glo = Time::<Glonass>::from_day_tod(
        5_000,
        DurationParts {
            seconds: 36_000,
            nanos: 0,
        },
    )
    .unwrap();
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
    let gps = Time::<Gps>::from_week_tow(
        2086,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();
    let via_trait: Time<Glonass> = gps.into_scale_with(ls).unwrap();
    let via_fn = gps_to_glonass(gps, ls).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_into_scale_with_glonass_gps_matches_glonass_to_gps() {
    let ls = LeapSeconds::builtin();
    let glo = Time::<Glonass>::from_day_tod(
        8_000,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();
    let via_trait: Time<Gps> = glo.into_scale_with(ls).unwrap();
    let via_fn = glonass_to_gps(glo, ls).unwrap();

    assert_eq!(via_trait, via_fn);
}

#[test]
fn test_glonass_display_canonical_format() {
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
fn test_glonass_display_epoch_is_day_zero() {
    assert_eq!(Time::<Glonass>::EPOCH.to_string(), "GLO 0:00000.000");
}

#[test]
fn test_glonass_display_tod_zero_padded_to_5_digits() {
    let t = Time::<Glonass>::from_day_tod(
        1,
        DurationParts {
            seconds: 1,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.to_string(), "GLO 1:00001.000");
}

#[test]
fn test_glonass_day_accessor_large_value() {
    let t = Time::<Glonass>::from_day_tod(
        99_999,
        DurationParts {
            seconds: 86_399,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.day(), 99_999);
    assert_eq!(t.tod_seconds(), 86_399);
}

// GLONASS does not roll over every 7 days, like GPS (every 604,800
// seconds/week). The day counter monotonically increases from the epoch.
#[test]
fn test_glonass_day_counter_does_not_rollover_at_7() {
    // Check that days 7, 14, 21, ... give the correct day_of_week,
    // but access to `day()` does NOT perform a cyclic reset (does not wrap).
    for n in [7u32, 14, 21, 100, 1000] {
        let t = Time::<Glonass>::from_day_tod(
            n,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t.day(), n, "day() should return raw day count, not wrapped");
    }
}

// Unlike GPS, which uses a "week" + "TOW" structure, GLONASS uses an absolute
// day count from the epoch. Check that creation from large day values works
// correctly.
#[test]
fn test_glonass_large_day_count_roundtrip() {
    // ~30 years after the epoch ≈ 10 950 days
    let t = Time::<Glonass>::from_day_tod(
        10_950,
        DurationParts {
            seconds: 43_200,
            nanos: 0,
        },
    )
    .unwrap();

    assert_eq!(t.day(), 10_950);
    assert_eq!(t.tod_seconds(), 43_200);
}

// At the GLONASS epoch (1996-01-01 00:00:00 UTC(SU) =
// 1995-12-31 21:00:00 UTC): GPS_s = ? (derived from UTC)
//
// UTC at the GLONASS epoch: 757_371_600 s from 1972
// GPS = UTC − epoch difference + (TAI-UTC − 19)
//     = 757_371_600 − 252_892_800 + (30 − 19)  [TAI-UTC = 30 on 1996-01-01]
//     = 504_478_800 + 11 = 504_478_811
//
// Check: GPS_s = 504_478_811 → GPS week ≈ 833, TOW = some number
// of seconds
#[test]
fn test_glonass_epoch_in_gps_seconds() {
    let ls = LeapSeconds::builtin();

    // GLONASS epoch → UTC → GPS
    let glo_epoch = Time::<Glonass>::EPOCH;
    let utc: Time<Utc> = glo_epoch.into_scale().unwrap();
    let gps: Time<Gps> = utc.into_scale_with(ls).unwrap();

    // Expected: GPS_s = 757_371_600 - 252_892_800 + (30 - 19)
    // = 504_478_800 + 11 = 504_478_811
    assert_eq!(
        gps.as_seconds(),
        504_478_810,
        "GLONASS epoch expressed in GPS seconds"
    );
    // GPS week: 504_478_810 / 604_800 = 834 weeks + remainder
    assert_eq!(gps.week(), 834);
}

// GPS epoch (1980-01-06 00:00:00 UTC), expressed in GLONASS time.
//
// GPS epoch in UTC seconds = 252_892_800 (from the UTC epoch 1972-01-01)
// GPS epoch → UTC time (UTC epoch = 252_892_800 s from 1972)
// UTC → GLO: GLO_ns = UTC_ns - 757_371_600_000_000_000
// UTC_ns at GPS epoch = 252_892_800_000_000_000
// GLO_ns = 252_892_800_000_000_000 - 757_371_600_000_000_000 = NEGATIVE
//
// This means the GPS epoch is earlier than the GLONASS epoch → overflow is
// expected.
#[test]
fn test_test_gps_epoch_predates_glonass_epoch() {
    // GPS epoch (1980-01-06) is earlier than the GLONASS epoch (1996-01-01),
    // therefore converting the GPS epoch to GLONASS should fail with
    // Overflow
    let ls = LeapSeconds::builtin();
    let gps_epoch = Time::<Gps>::EPOCH;
    let result: Result<Time<Glonass>, _> = gps_epoch.into_scale_with(ls);

    assert!(
        matches!(result, Err(GnssTimeError::Overflow)),
        "GPS epoch (1980) is before GLONASS epoch (1996) → should be overflow"
    );
}
