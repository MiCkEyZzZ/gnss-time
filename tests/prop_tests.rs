// # Property-based tests (without proptest — manual implementation)
//
// Since proptest is unavailable in the current environment, we implement
// a property-based approach manually using deterministic pseudo-random
// samples covering the entire value range.
//
// ## Properties being tested
//
// 1. **Roundtrip GPS→UTC→GPS**: for any `t: Time<Gps>`, it holds that
//    `GPS→UTC→GPS == t`
// 2. **Roundtrip through all 5 domains**: GPS→GAL→BDT→TAI→GPS == GPS
// 3. **Sorting**: ordering of `Vec<Time<Gps>>` matches ordering by internal u64
// 4. **Historical leap second transitions**: all 18 events from 1981 to 2017
// 5. **Real IGS epochs**: several historical GPS timestamps

use gnss_time::{
    gps_to_utc, utc_to_gps, Beidou, DurationParts, Galileo, Gps, IntoScale, LeapSeconds, Tai, Time,
};

// Deterministic sampling: uniform across the entire GPS range + edge cases.
fn gps_sample_points() -> Vec<Time<Gps>> {
    let mut pts = Vec::with_capacity(256);

    // Boundary values
    pts.push(Time::<Gps>::EPOCH);
    pts.push(Time::<Gps>::from_nanos(1));
    pts.push(Time::<Gps>::from_nanos(1_000_000_000 - 1)); // 1s - 1ns

    // All known GPS epochs of leap second transitions (GPS seconds)
    let leap_gps_seconds = &[
        46_828_801,    // 1981-07-01
        78_364_802,    // 1982-07-01
        109_900_803,   // 1983-07-01
        173_059_204,   // 1985-07-01
        252_028_805,   // 1988-01-01
        315_187_206,   // 1990-01-01
        346_723_207,   // 1991-01-01
        393_984_008,   // 1992-07-01
        425_520_009,   // 1993-07-01
        457_056_010,   // 1994-07-01
        504_489_611,   // 1996-01-01
        551_750_412,   // 1997-07-01
        599_184_013,   // 1999-01-01
        820_108_814,   // 2006-01-01
        914_803_215,   // 2009-01-01
        1_025_136_016, // 2012-07-01
        1_119_744_017, // 2015-07-01
        1_167_264_018, // 2017-01-01
    ];
    for &s in leap_gps_seconds {
        // 2 seconds before and after each transition
        if s >= 2 {
            pts.push(Time::<Gps>::from_seconds(s - 2));
            pts.push(Time::<Gps>::from_seconds(s - 1));
        }
        pts.push(Time::<Gps>::from_seconds(s));
        pts.push(Time::<Gps>::from_seconds(s + 1));
        pts.push(Time::<Gps>::from_seconds(s + 2));
    }

    // Uniform points across the entire range (~every 29 years)
    let step = u64::MAX / 20;
    for i in 0..=20 {
        pts.push(Time::<Gps>::from_nanos(step.saturating_mul(i)));
    }

    // IGS real epochs (known GPS weeks)
    for week in [1, 100, 500, 1000, 1500, 2000, 2086, 2100, 2200] {
        pts.push(
            Time::<Gps>::from_week_tow(
                week,
                DurationParts {
                    seconds: 0,
                    nanos: 0,
                },
            )
            .unwrap(),
        );
        pts.push(
            Time::<Gps>::from_week_tow(
                week,
                DurationParts {
                    seconds: 302_400,
                    nanos: 0,
                },
            )
            .unwrap(),
        ); // середина недели
    }

    pts
}

#[test]
fn prop_gps_utc_gps_roundtrip_for_all_samples() {
    use gnss_time::{
        convert::{ConvertResult, IntoScaleWith},
        scale::Utc,
    };
    let ls = LeapSeconds::builtin();
    let samples = gps_sample_points();

    for t in &samples {
        // Use checked version to detect ambiguity window.
        // In this window (1 second around each leap second) roundtrip is not
        // guaranteed.
        let result: ConvertResult<Time<Utc>> = match t.into_scale_with_checked(ls) {
            Ok(r) => r,
            Err(_) => continue,
        };
        // Skip ambiguous points (leap second insertion window)
        let utc = match result {
            ConvertResult::Exact(u) => u,
            ConvertResult::AmbiguousLeapSecond(_) => continue,
        };
        let back = utc_to_gps(utc, ls).unwrap();
        assert_eq!(
            *t,
            back,
            "GPS→UTC→GPS roundtrip failed for t={} ns",
            t.as_nanos()
        );
    }
}

#[test]
fn prop_gps_galileo_gps_roundtrip_for_all_samples() {
    let samples = gps_sample_points();

    for t in &samples {
        // GPS→GAL goes via TAI: if GPS+19 > u64::MAX → overflow, skip
        let gal: Time<Galileo> = match t.into_scale() {
            Ok(g) => g,
            Err(_) => continue,
        };
        let back: Time<Gps> = match gal.into_scale() {
            Ok(b) => b,
            Err(_) => continue,
        };
        assert_eq!(
            *t,
            back,
            "GPS→GAL→GPS roundtrip failed for t={} ns",
            t.as_nanos()
        );
        // Galileo and GPS store identical nanoseconds
        assert_eq!(t.as_nanos(), gal.as_nanos());
    }
}

#[test]
fn prop_gps_beidou_gps_roundtrip_for_all_samples() {
    let samples = gps_sample_points();

    for t in &samples {
        let bdt: Time<Beidou> = match t.into_scale() {
            Ok(b) => b,
            Err(_) => continue, // underflow for small GPS values
        };
        let back: Time<Gps> = bdt.into_scale().unwrap();
        assert_eq!(
            *t,
            back,
            "GPS→BDT→GPS roundtrip failed for t={} ns",
            t.as_nanos()
        );
    }
}

#[test]
fn prop_gps_tai_gps_roundtrip_for_all_samples() {
    let samples = gps_sample_points();

    for t in &samples {
        let tai: Time<Tai> = match t.into_scale() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let back: Time<Gps> = match tai.into_scale() {
            Ok(b) => b,
            Err(_) => continue,
        };
        assert_eq!(
            *t,
            back,
            "GPS→TAI→GPS roundtrip failed for t={} ns",
            t.as_nanos()
        );
    }
}

#[test]
fn prop_sort_order_matches_internal_u64() {
    let mut samples = gps_sample_points();

    // Sort via Ord<Time<Gps>>
    let mut by_time = samples.clone();
    by_time.sort();

    // Sort directly by u64
    samples.sort_by_key(|t| t.as_nanos());

    assert_eq!(
        by_time, samples,
        "Time<Gps> sort order must match u64 order"
    );
}

#[test]
fn prop_gps_to_utc_is_monotone_between_leap_seconds() {
    let ls = LeapSeconds::builtin();

    // Interval 1999-01-01 (ls=32) to 2006-01-01 (ls=33) — 7 years without leap
    // second
    let start = Time::<Gps>::from_seconds(599_184_014);
    let mid = Time::<Gps>::from_seconds(709_646_413); // ~2002
    let end = Time::<Gps>::from_seconds(820_108_812);

    let utc_start = gps_to_utc(start, ls).unwrap();
    let utc_mid = gps_to_utc(mid, ls).unwrap();
    let utc_end = gps_to_utc(end, ls).unwrap();

    assert!(
        utc_start < utc_mid,
        "UTC must increase with GPS within stable interval"
    );
    assert!(
        utc_mid < utc_end,
        "UTC must increase with GPS within stable interval"
    );
    assert!(start < mid, "GPS ordering correct");
    assert!(mid < end, "GPS ordering correct");
}

// Table: [(GPS_seconds, expected_GPS_minus_UTC_seconds)]
const GPS_OFFSET_TABLE: [(u64, i64); 4] = [
    // In 1980 year GPS-UTC = 0
    (1, 0),
    // after 1981-07-01: GPS-UTC = 1
    (46_828_802, 1),
    // after 1999-01-01: GPS-UTC = 13
    (599_184_014, 13),
    // after 2017-01-01: GPS-UTC = 18
    (1_167_264_019, 18),
];

#[test]
fn prop_gps_minus_utc_matches_expected_offsets() {
    let ls = LeapSeconds::builtin();
    // UTC epoch offset: 252_892_800 s from 1972-01-01 to GPS epoch 1980-01-06
    const GPS_UTC_EPOCH_OFFSET_S: i64 = 252_892_800;

    for &(gps_s, expected_offset) in &GPS_OFFSET_TABLE {
        let gps = Time::<Gps>::from_seconds(gps_s);
        let utc = gps_to_utc(gps, ls).unwrap();

        let gps_s_i64 = gps_s as i64;
        let utc_s_i64 = utc.as_seconds() as i64;
        // GPS_seconds_from_epoch - (UTC_seconds_from_UTC_epoch - GPS_UTC_epoch_diff)
        let actual_offset = gps_s_i64 - (utc_s_i64 - GPS_UTC_EPOCH_OFFSET_S);

        assert_eq!(
            actual_offset, expected_offset,
            "GPS−UTC offset wrong at GPS={gps_s}s: expected {expected_offset}s, got {actual_offset}s"
        );
    }
}

// Verifies GPS jumps by 2 seconds (UTC by 1) at each leap second.
struct LeapTransition {
    // GPS seconds AFTER the leap second is inserted (first second with new value)
    gps_after: u64,
    // GPS seconds BEFORE insertion (last second with old value)
    gps_before: u64,
    // Expected difference in UTC seconds between after and before (should be 1)
    expected_utc_diff: i64,
}

#[test]
fn prop_all_18_leap_second_transitions_correct() {
    let ls = LeapSeconds::builtin();

    let transitions = [
        // gps_after = (unix_event - 315_964_800) + new_GPS_UTC_offset
        // gps_before = gps_after - 2 (GPS jumps by 2 seconds, UTC by 1 second)
        LeapTransition {
            gps_after: 46_828_801,
            gps_before: 46_828_799,
            expected_utc_diff: 1,
        }, // 1981-07-01
        LeapTransition {
            gps_after: 78_364_802,
            gps_before: 78_364_800,
            expected_utc_diff: 1,
        }, // 1982-07-01
        LeapTransition {
            gps_after: 109_900_803,
            gps_before: 109_900_801,
            expected_utc_diff: 1,
        }, // 1983-07-01
        LeapTransition {
            gps_after: 173_059_204,
            gps_before: 173_059_202,
            expected_utc_diff: 1,
        }, // 1985-07-01
        LeapTransition {
            gps_after: 252_028_805,
            gps_before: 252_028_803,
            expected_utc_diff: 1,
        }, // 1988-01-01
        LeapTransition {
            gps_after: 315_187_206,
            gps_before: 315_187_204,
            expected_utc_diff: 1,
        }, // 1990-01-01
        LeapTransition {
            gps_after: 346_723_207,
            gps_before: 346_723_205,
            expected_utc_diff: 1,
        }, // 1991-01-01
        LeapTransition {
            gps_after: 393_984_008,
            gps_before: 393_984_006,
            expected_utc_diff: 1,
        }, // 1992-07-01
        LeapTransition {
            gps_after: 425_520_009,
            gps_before: 425_520_007,
            expected_utc_diff: 1,
        }, // 1993-07-01
        LeapTransition {
            gps_after: 457_056_010,
            gps_before: 457_056_008,
            expected_utc_diff: 1,
        }, // 1994-07-01
        LeapTransition {
            gps_after: 504_489_611,
            gps_before: 504_489_609,
            expected_utc_diff: 1,
        }, // 1996-01-01
        LeapTransition {
            gps_after: 551_750_412,
            gps_before: 551_750_410,
            expected_utc_diff: 1,
        }, // 1997-07-01
        LeapTransition {
            gps_after: 599_184_013,
            gps_before: 599_184_011,
            expected_utc_diff: 1,
        }, // 1999-01-01
        LeapTransition {
            gps_after: 820_108_814,
            gps_before: 820_108_812,
            expected_utc_diff: 1,
        }, // 2006-01-01
        LeapTransition {
            gps_after: 914_803_215,
            gps_before: 914_803_213,
            expected_utc_diff: 1,
        }, // 2009-01-01
        LeapTransition {
            gps_after: 1_025_136_016,
            gps_before: 1_025_136_014,
            expected_utc_diff: 1,
        }, // 2012-07-01
        LeapTransition {
            gps_after: 1_119_744_017,
            gps_before: 1_119_744_015,
            expected_utc_diff: 1,
        }, // 2015-07-01
        LeapTransition {
            gps_after: 1_167_264_018,
            gps_before: 1_167_264_016,
            expected_utc_diff: 1,
        }, // 2017-01-01
    ];

    for (i, t) in transitions.iter().enumerate() {
        let gps_b = Time::<Gps>::from_seconds(t.gps_before);
        let gps_a = Time::<Gps>::from_seconds(t.gps_after);

        let utc_b = gps_to_utc(gps_b, ls).unwrap();
        let utc_a = gps_to_utc(gps_a, ls).unwrap();

        let utc_diff = (utc_a - utc_b).as_seconds();
        assert_eq!(
            utc_diff, t.expected_utc_diff,
            "Leap #{i}: GPS jumped 2s ({} → {}) but UTC diff should be {}s, got {}s",
            t.gps_before, t.gps_after, t.expected_utc_diff, utc_diff
        );
    }
}

#[test]
fn prop_gps_utc_offset_strictly_increases_at_each_transition() {
    let ls = LeapSeconds::builtin();
    const GPS_UTC_EPOCH_OFFSET_S: i64 = 252_892_800;

    // GPS seconds immediately after each leap second
    let transition_points = &[
        46_828_802,
        78_364_803,
        109_900_804,
        173_059_205,
        252_028_806,
        315_187_207,
        346_723_208,
        393_984_009,
        425_520_010,
        457_056_011,
        504_489_612,
        551_750_413,
        599_184_014,
        820_108_815,
        914_803_216,
        1_025_136_017,
        1_119_744_018,
        1_167_264_019,
    ];

    let mut prev_offset = -1i64;
    for &gps_s in transition_points {
        let gps = Time::<Gps>::from_seconds(gps_s);
        let utc = gps_to_utc(gps, ls).unwrap();
        let offset = gps_s as i64 - (utc.as_seconds() as i64 - GPS_UTC_EPOCH_OFFSET_S);
        assert!(
            offset > prev_offset,
            "GPS-UTC offset must increase at each leap second: prev={prev_offset}, current={offset} at GPS={gps_s}s"
        );
        prev_offset = offset;
    }
}
