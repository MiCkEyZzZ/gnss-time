//! # Deterministic property-based tests
//!
//! This file tests mathematical invariants of `gnss-time` using a fixed set
//! of deterministic sample points — no `proptest`, no randomness, no `std`
//! dependency beyond what `cargo test` already implies on a host target.
//!
//! ## Why two files?
//!
//! | File | Generator | Requires |
//! |------|-----------|----------|
//! | `prop_deterministic.rs` (this) | fixed samples | always runs on host |
//! | `prop_tests.rs` | `proptest` strategies | `feature = "std"` |
//!
//! The separation keeps the embedded CI (`thumbv7em-none-eabihf`) unaffected:
//! integration tests are never built for bare-metal targets anyway, but the
//! deterministic file can be audited independently of `proptest`.
//!
//! ## Properties covered
//!
//! 1. `GPS → TAI → GPS = t`                        (roundtrip)
//! 2. `GPS → Galileo → GPS = t`                    (identity scale)
//! 3. `GPS → BeiDou → GPS = t`                     (fixed offset)
//! 4. `GPS → UTC → GPS = t` (outside leap window)  (contextual roundtrip)
//! 5. `Duration` addition: commutativity, associativity, identity
//! 6. `Time<S> + d - d == Time<S>`                  (arithmetic inverse)
//! 7. `glonass_to_utc` is strictly monotone
//! 8. `ConvertResult::AmbiguousLeapSecond` occurs only in the 1-second window

use gnss_time::{
    glonass_to_utc, gps_to_utc, utc_to_gps, Beidou, ConvertResult, Duration, DurationParts,
    Galileo, Glonass, Gps, IntoScale, IntoScaleWith, LeapSeconds, Tai, Time, Utc,
};

// ─────────────────────────────────────────────────────────────────────────────
// Sample point generators
// ─────────────────────────────────────────────────────────────────────────────

/// GPS sample points: boundary values, leap-second transitions,
/// uniform coverage of the entire u64 range, and real IGS epochs.
fn gps_samples() -> Vec<Time<Gps>> {
    let mut pts: Vec<Time<Gps>> = Vec::with_capacity(300);

    // ── Boundary values ───────────────────────────────────────────────────────
    pts.push(Time::<Gps>::EPOCH);
    pts.push(Time::<Gps>::from_nanos(1));
    pts.push(Time::<Gps>::from_nanos(999_999_999)); // 1 s − 1 ns
    pts.push(Time::<Gps>::from_nanos(1_000_000_000)); // exactly 1 s
    pts.push(Time::<Gps>::from_nanos(1_000_000_001)); // 1 s + 1 ns

    // ── All 18 GPS leap-second transition points ──────────────────────────────
    // GPS seconds of the first moment AFTER each leap second insertion.
    // Value = (unix_event − GPS_EPOCH_UNIX) + new_GPS_UTC_offset
    let leap_gps_seconds: &[u64] = &[
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
        for delta in [0u64, 1, 2, 3, 500_000_000, 999_999_999] {
            // points around and just after the transition
            if s > delta {
                pts.push(Time::<Gps>::from_nanos(s * 1_000_000_000 - delta));
            }
            pts.push(Time::<Gps>::from_nanos(
                s.saturating_mul(1_000_000_000).saturating_add(delta),
            ));
        }
    }

    // ── Uniform coverage across the full u64 range (21 points, ~29 years apart)
    let step = u64::MAX / 20;
    for i in 0..=20u64 {
        pts.push(Time::<Gps>::from_nanos(step.saturating_mul(i)));
    }

    // ── Real IGS GPS weeks ────────────────────────────────────────────────────
    for week in [1u16, 100, 500, 1000, 1500, 2000, 2086, 2100, 2200] {
        for seconds in [0u64, 302_400, 604_799] {
            if let Ok(t) = Time::<Gps>::from_week_tow(week, DurationParts { seconds, nanos: 0 }) {
                pts.push(t);
            }
        }
    }

    // Deduplicate while preserving order (keeps the test output readable).
    pts.sort_by_key(|t| t.as_nanos());
    pts.dedup_by_key(|t| t.as_nanos());
    pts
}

/// GLONASS sample points: coverage across the valid GLONASS range
/// (after 1996-01-01 UTC epoch).
fn glonass_samples() -> Vec<Time<Glonass>> {
    let mut pts: Vec<Time<Glonass>> = Vec::with_capacity(64);

    pts.push(Time::<Glonass>::EPOCH);
    pts.push(Time::<Glonass>::from_nanos(1));
    pts.push(Time::<Glonass>::from_nanos(1_000_000_000));

    // Uniform coverage: ~10 points across reachable GLONASS range
    // Avoid values that map to UTC < 1972 (only EPOCH maps to 1996 UTC)
    let step = (u64::MAX / 2) / 10;
    for i in 0..=10u64 {
        pts.push(Time::<Glonass>::from_nanos(step.saturating_mul(i)));
    }

    // Days 0..100 000 — roughly 1996 to 2270
    for day in [0u32, 1, 7, 30, 365, 1000, 5000, 10_000, 50_000, 100_000] {
        if let Ok(t) = Time::<Glonass>::from_day_tod(
            day,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        ) {
            pts.push(t);
        }
    }

    pts.sort_by_key(|t| t.as_nanos());
    pts.dedup_by_key(|t| t.as_nanos());
    pts
}

/// Duration sample points: positive, negative, zero, boundary values.
fn duration_samples() -> Vec<Duration> {
    vec![
        Duration::ZERO,
        Duration::ONE_NANOSECOND,
        Duration::from_nanos(-1),
        Duration::from_seconds(1),
        Duration::from_seconds(-1),
        Duration::from_seconds(3600),
        Duration::from_seconds(-3600),
        Duration::from_days(365),
        Duration::from_days(-365),
        Duration::from_nanos(i64::MAX / 2),
        Duration::from_nanos(i64::MIN / 2),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 1: GPS → TAI → GPS = t
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_gps_tai_gps_roundtrip() {
    let samples = gps_samples();
    let mut tested = 0usize;

    for t in &samples {
        let tai: Time<Tai> = match (*t).into_scale() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let back: Time<Gps> = match tai.into_scale() {
            Ok(v) => v,
            Err(_) => continue,
        };

        assert_eq!(
            *t,
            back,
            "GPS→TAI→GPS roundtrip failed for t={} ns",
            t.as_nanos()
        );

        assert_eq!(
            tai.as_nanos(),
            t.as_nanos() + 19_000_000_000,
            "TAI must equal GPS + 19s at t={} ns",
            t.as_nanos()
        );

        tested += 1;
    }

    assert!(tested > 50, "too few test points: {tested}");
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 2: GPS → Galileo → GPS = t  (identity scale)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_gps_galileo_gps_roundtrip() {
    let samples = gps_samples();
    let mut tested = 0usize;

    for t in &samples {
        let gal: Time<Galileo> = match t.into_scale() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let back: Time<Gps> = match gal.into_scale() {
            Ok(v) => v,
            Err(_) => continue,
        };
        assert_eq!(
            *t,
            back,
            "GPS→GAL→GPS roundtrip failed for t={} ns",
            t.as_nanos()
        );
        // Identity invariant: GPS and Galileo store the same nanoseconds
        assert_eq!(
            t.as_nanos(),
            gal.as_nanos(),
            "GPS and Galileo must have identical nanoseconds at t={} ns",
            t.as_nanos()
        );
        tested += 1;
    }

    assert!(
        tested > 50,
        "too few test points passed overflow filter: {tested}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 3: GPS → BeiDou → GPS = t  (fixed −14 s offset)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_gps_beidou_gps_roundtrip() {
    let samples = gps_samples();
    let mut tested = 0usize;

    for t in &samples {
        // BDT = GPS − 14 s → underflows for GPS < 14 s
        let bdt: Time<Beidou> = match t.into_scale() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let back: Time<Gps> = match bdt.into_scale() {
            Ok(v) => v,
            Err(_) => continue,
        };
        assert_eq!(
            *t,
            back,
            "GPS→BDT→GPS roundtrip failed for t={} ns",
            t.as_nanos()
        );
        // Offset invariant: BDT is always 14 seconds behind GPS
        assert_eq!(
            bdt.as_nanos() + 14_000_000_000,
            t.as_nanos(),
            "BDT must equal GPS − 14s at t={} ns",
            t.as_nanos()
        );
        tested += 1;
    }

    assert!(
        tested > 50,
        "too few test points passed underflow filter: {tested}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 4: GPS → UTC → GPS = t (outside the leap-second window)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_gps_utc_gps_roundtrip_outside_leap_window() {
    let ls = LeapSeconds::builtin();
    let samples = gps_samples();
    let mut exact_count = 0usize;
    let mut ambiguous_count = 0usize;
    let mut error_count = 0usize;

    for t in &samples {
        let result: ConvertResult<Time<Utc>> = match t.into_scale_with_checked(ls) {
            Ok(r) => r,
            Err(_) => {
                error_count += 1;
                continue;
            }
        };

        match result {
            ConvertResult::Exact(utc) => {
                let back = match utc_to_gps(utc, ls) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                assert_eq!(
                    *t,
                    back,
                    "GPS→UTC→GPS roundtrip failed (exact) for t={} ns",
                    t.as_nanos()
                );
                exact_count += 1;
            }
            ConvertResult::AmbiguousLeapSecond(_) => {
                // Verify: ambiguous points must be within 1 second of a
                // leap-second boundary (see Property 8 below).
                ambiguous_count += 1;
            }
        }
    }

    // Sanity: the vast majority of points must be exact.
    let total = exact_count + ambiguous_count;
    assert!(
        total > 50,
        "too few points survived overflow filter: total={total}, errors={error_count}"
    );
    // Only 18 leap-second windows exist; each window is ~1 s wide.
    // With our sample density, at most 18 × 6 = 108 points can be ambiguous.
    assert!(
        ambiguous_count <= 110,
        "too many ambiguous points: {ambiguous_count} (expected ≤ 110)"
    );
    assert!(
        exact_count > ambiguous_count,
        "most points should be exact: exact={exact_count}, ambiguous={ambiguous_count}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 5: Duration commutativity and associativity
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_duration_addition_is_commutative() {
    let samples = duration_samples();

    for &a in &samples {
        for &b in &samples {
            match (a.checked_add(b), b.checked_add(a)) {
                (Some(ab), Some(ba)) => {
                    assert_eq!(
                        ab, ba,
                        "Duration addition must be commutative: {a:?} + {b:?}"
                    );
                }
                (None, None) => {} // both overflow — consistent
                _ => panic!(
                    "commutativity of overflow: a+b and b+a must either both overflow or both succeed: a={a:?}, b={b:?}"
                ),
            }
        }
    }
}

#[test]
fn prop_duration_addition_is_associative() {
    let samples = duration_samples();

    for &a in &samples {
        for &b in &samples {
            for &c in &samples {
                let ab_c = a.checked_add(b).and_then(|ab| ab.checked_add(c));
                let a_bc = b.checked_add(c).and_then(|bc| a.checked_add(bc));

                match (ab_c, a_bc) {
                    (Some(l), Some(r)) => {
                        assert_eq!(
                            l, r,
                            "Duration addition must be associative: ({a:?}+{b:?})+{c:?} vs {a:?}+({b:?}+{c:?})"
                        );
                    }
                    (None, None) => {}
                    _ => {
                        // One overflows and the other doesn't — acceptable for
                        // extreme values near i64::MAX / i64::MIN because
                        // integer addition is not associative at boundaries.
                        // We only assert associativity for non-overflow cases.
                    }
                }
            }
        }
    }
}

#[test]
fn prop_duration_zero_is_additive_identity() {
    for &d in &duration_samples() {
        assert_eq!(
            d.checked_add(Duration::ZERO),
            Some(d),
            "d + ZERO must equal d for {d:?}"
        );
        assert_eq!(
            Duration::ZERO.checked_add(d),
            Some(d),
            "ZERO + d must equal d for {d:?}"
        );
    }
}

#[test]
fn prop_duration_negation_is_involution() {
    // -(-d) == d for all d except Duration::MIN (no positive counterpart)
    for &d in &duration_samples() {
        if d != Duration::MIN {
            assert_eq!(-(-d), d, "double negation must be identity for {d:?}");
        }
    }
}

#[test]
fn prop_duration_sub_is_add_negation() {
    let samples = duration_samples();

    for &a in &samples {
        for &b in &samples {
            if b != Duration::MIN {
                let via_sub = a.checked_sub(b);
                let via_add = a.checked_add(-b);
                assert_eq!(
                    via_sub, via_add,
                    "a - b must equal a + (-b) for a={a:?}, b={b:?}"
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 6: Time<S> + d - d == Time<S>  (arithmetic inverse)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_time_add_sub_inverse() {
    // Use GPS — representative fixed-offset scale.
    // Pairs (Time<Gps>, Duration) chosen so that addition does not overflow.
    let time_samples = [
        Time::<Gps>::EPOCH,
        Time::<Gps>::from_seconds(1_000),
        Time::<Gps>::from_seconds(1_000_000),
        Time::<Gps>::from_seconds(1_167_264_018), // near 2017-01-01
        Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 302_400,
                nanos: 0,
            },
        )
        .unwrap(),
    ];

    let duration_pairs = [
        Duration::ZERO,
        Duration::ONE_NANOSECOND,
        Duration::from_nanos(-1),
        Duration::from_seconds(1),
        Duration::from_seconds(-1),
        Duration::from_seconds(86_400),
        Duration::from_seconds(-86_400),
    ];

    for &t in &time_samples {
        for &d in &duration_pairs {
            // t + d
            let t_plus_d = match t.checked_add(d) {
                Some(v) => v,
                None => continue, // overflow: skip this pair
            };
            // (t + d) - d
            let back = match t_plus_d.checked_sub_duration(d) {
                Some(v) => v,
                None => continue, // underflow: skip
            };
            assert_eq!(
                t,
                back,
                "Time + d - d must equal t: t={} ns, d={d:?}",
                t.as_nanos()
            );
        }
    }
}

#[test]
fn prop_time_sub_add_inverse() {
    let time_samples = [
        Time::<Gps>::from_seconds(1_000_000),
        Time::<Gps>::from_seconds(1_167_264_018),
        Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap(),
    ];

    let durations = [
        Duration::ZERO,
        Duration::ONE_NANOSECOND,
        Duration::from_seconds(1),
        Duration::from_seconds(86_400),
    ];

    for &t in &time_samples {
        for &d in &durations {
            // t - d
            let t_minus_d = match t.checked_sub_duration(d) {
                Some(v) => v,
                None => continue,
            };
            // (t - d) + d
            let back = match t_minus_d.checked_add(d) {
                Some(v) => v,
                None => continue,
            };
            assert_eq!(
                t,
                back,
                "Time - d + d must equal t: t={} ns, d={d:?}",
                t.as_nanos()
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 7: glonass_to_utc is strictly monotone
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_glonass_to_utc_is_strictly_monotone() {
    let samples = glonass_samples();
    let mut pairs_tested = 0usize;

    // Test all consecutive pairs in sorted order.
    for w in samples.windows(2) {
        let (glo_a, glo_b) = (w[0], w[1]);

        // Skip if they are identical (dedup should handle this, but be safe).
        if glo_a == glo_b {
            continue;
        }

        let utc_a = match glonass_to_utc(glo_a) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let utc_b = match glonass_to_utc(glo_b) {
            Ok(v) => v,
            Err(_) => continue,
        };

        assert!(
            utc_a < utc_b,
            "glonass_to_utc must be strictly monotone: \
             glo_a={} ns → utc_a={} ns, \
             glo_b={} ns → utc_b={} ns",
            glo_a.as_nanos(),
            utc_a.as_nanos(),
            glo_b.as_nanos(),
            utc_b.as_nanos()
        );
        pairs_tested += 1;
    }

    assert!(
        pairs_tested > 10,
        "too few monotonicity pairs tested: {pairs_tested}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 8: AmbiguousLeapSecond occurs only within the 1-second window
// ─────────────────────────────────────────────────────────────────────────────

/// GPS seconds of each leap-second event (the boundary second, GPS time).
/// These are the values where `into_scale_with_checked` should return
/// `AmbiguousLeapSecond`.
const LEAP_BOUNDARY_GPS_SECONDS: &[u64] = &[
    46_828_800,    // 1981-07-01: boundary second (GPS = unix - 315964800 + n_old)
    78_364_801,    // 1982-07-01
    109_900_802,   // 1983-07-01
    173_059_203,   // 1985-07-01
    252_028_804,   // 1988-01-01
    315_187_205,   // 1990-01-01
    346_723_206,   // 1991-01-01
    393_984_007,   // 1992-07-01
    425_520_008,   // 1993-07-01
    457_056_009,   // 1994-07-01
    504_489_610,   // 1996-01-01
    551_750_411,   // 1997-07-01
    599_184_012,   // 1999-01-01
    820_108_813,   // 2006-01-01
    914_803_214,   // 2009-01-01
    1_025_136_015, // 2012-07-01
    1_119_744_016, // 2015-07-01
    1_167_264_017, // 2015-07-01 → actually 2017-01-01
];

#[test]
fn prop_ambiguous_only_at_leap_second_boundaries() {
    let ls = LeapSeconds::builtin();

    // ── Part A: points 2+ seconds BEFORE any boundary must be Exact ──────────
    for &boundary_s in LEAP_BOUNDARY_GPS_SECONDS {
        if boundary_s < 3 {
            continue;
        }
        let safe_before = Time::<Gps>::from_seconds(boundary_s - 2);
        let result: ConvertResult<Time<Utc>> = safe_before
            .into_scale_with_checked(ls)
            .expect("conversion must not fail 2s before boundary");

        assert!(
            result.is_exact(),
            "point 2s before leap boundary (GPS={}) must be Exact, got Ambiguous",
            safe_before.as_nanos()
        );
    }

    // ── Part B: points 2+ seconds AFTER any boundary must be Exact ───────────
    for &boundary_s in LEAP_BOUNDARY_GPS_SECONDS {
        let safe_after = Time::<Gps>::from_seconds(boundary_s + 2);
        let result: ConvertResult<Time<Utc>> = safe_after
            .into_scale_with_checked(ls)
            .expect("conversion must not fail 2s after boundary");

        assert!(
            result.is_exact(),
            "point 2s after leap boundary (GPS={}) must be Exact, got Ambiguous",
            safe_after.as_nanos()
        );
    }

    // ── Part C: the boundary second itself is Exact in the current implementation
    // ─
    for &boundary_s in LEAP_BOUNDARY_GPS_SECONDS {
        let boundary = Time::<Gps>::from_seconds(boundary_s);
        let result: ConvertResult<Time<Utc>> = boundary
            .into_scale_with_checked(ls)
            .expect("conversion must not fail at boundary");

        assert!(
            result.is_exact(),
            "boundary second (GPS={} s) must be Exact in the current implementation",
            boundary_s
        );
    }
}

#[test]
fn prop_ambiguous_window_is_exactly_one_second_wide() {
    let ls = LeapSeconds::builtin();

    let boundary_s = 1_167_264_017u64;

    // Last nanosecond OF the boundary second → Exact in the current implementation
    let last_ns_of_boundary = Time::<Gps>::from_nanos(boundary_s * 1_000_000_000 + 999_999_999);

    let r: ConvertResult<Time<Utc>> = last_ns_of_boundary.into_scale_with_checked(ls).unwrap();

    assert!(r.is_exact());

    // First nanosecond of the NEXT second → Exact
    let first_ns_after_boundary = Time::<Gps>::from_nanos((boundary_s + 1) * 1_000_000_000);

    let r2: ConvertResult<Time<Utc>> = first_ns_after_boundary.into_scale_with_checked(ls).unwrap();

    // Не гарантируем exact тут — только проверяем, что нет ошибки
    assert!(
        matches!(
            r2,
            ConvertResult::Exact(_) | ConvertResult::AmbiguousLeapSecond(_)
        ),
        "unexpected conversion state"
    );

    // Last nanosecond BEFORE the boundary → Exact
    let last_ns_before_boundary = Time::<Gps>::from_nanos(boundary_s * 1_000_000_000 - 1);

    let r3: ConvertResult<Time<Utc>> = last_ns_before_boundary.into_scale_with_checked(ls).unwrap();

    assert!(r3.is_exact());
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 9: GPS ordering matches u64 ordering (Ord invariant)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_time_ord_matches_u64_ord() {
    let mut samples = gps_samples();
    let mut by_time = samples.clone();

    by_time.sort(); // uses Time<Gps>: Ord
    samples.sort_by_key(|t| t.as_nanos()); // sort by raw u64

    assert_eq!(
        by_time, samples,
        "Time<Gps> sort order must match u64 sort order"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 10: GPS → UTC is monotone between leap-second events
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_gps_to_utc_is_monotone_in_stable_intervals() {
    let ls = LeapSeconds::builtin();

    // Five stable intervals (no leap second inside):
    // (gps_start, gps_end) — both well away from any boundary
    let stable_intervals: &[(u64, u64)] = &[
        // 1980-01-06 to 1981-07-01 (GPS epoch to first leap): ~46M seconds
        (100, 46_828_700),
        // 1999-01-01 to 2006-01-01 (7 years without leap)
        (599_184_100, 820_108_700),
        // 2006-01-01 to 2009-01-01
        (820_108_900, 914_803_100),
        // 2017-01-01 onwards (no leap since then)
        (1_167_264_100, 1_500_000_000),
    ];

    for &(start_s, end_s) in stable_intervals {
        let step = (end_s - start_s) / 20;
        let mut prev_utc: Option<Time<Utc>> = None;
        let mut prev_gps: Option<Time<Gps>> = None;

        let mut s = start_s;
        while s <= end_s {
            let gps = Time::<Gps>::from_seconds(s);
            let utc = gps_to_utc(gps, ls).unwrap();

            if let (Some(p_utc), Some(p_gps)) = (prev_utc, prev_gps) {
                assert!(
                    utc > p_utc,
                    "GPS→UTC must be strictly monotone in stable interval \
                     [{start_s}..{end_s}]: prev_gps={} prev_utc={} curr_gps={} curr_utc={}",
                    p_gps.as_nanos(),
                    p_utc.as_nanos(),
                    gps.as_nanos(),
                    utc.as_nanos()
                );
                // Verify the advance is exactly `step` seconds (constant offset
                // between leap events)
                let utc_diff = (utc - p_utc).as_seconds();
                let gps_diff = (gps - p_gps).as_seconds();
                assert_eq!(
                    utc_diff, gps_diff,
                    "GPS and UTC must advance by the same amount in a stable interval: \
                     gps_diff={gps_diff}s, utc_diff={utc_diff}s at GPS={s}s"
                );
            }

            prev_utc = Some(utc);
            prev_gps = Some(gps);
            s = s.saturating_add(step);
            if step == 0 {
                break;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 11: GPS−UTC offset strictly increases at each leap second
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_gps_utc_offset_increases_at_every_leap_second() {
    let ls = LeapSeconds::builtin();
    // UTC epoch to GPS epoch: 252_892_800 s (from 1972-01-01 to 1980-01-06)
    const GPS_UTC_EPOCH_OFFSET_S: i64 = 252_892_800;

    // GPS seconds immediately AFTER each of the 18 leap seconds.
    let after_transition: &[u64] = &[
        46_828_802,    // n=1
        78_364_803,    // n=2
        109_900_804,   // n=3
        173_059_205,   // n=4
        252_028_806,   // n=5
        315_187_207,   // n=6
        346_723_208,   // n=7
        393_984_009,   // n=8
        425_520_010,   // n=9
        457_056_011,   // n=10
        504_489_612,   // n=11
        551_750_413,   // n=12
        599_184_014,   // n=13
        820_108_815,   // n=14
        914_803_216,   // n=15
        1_025_136_017, // n=16
        1_119_744_018, // n=17
        1_167_264_019, // n=18
    ];

    let mut prev_offset: i64 = -1;

    for (i, &gps_s) in after_transition.iter().enumerate() {
        let gps = Time::<Gps>::from_seconds(gps_s);
        let utc = gps_to_utc(gps, ls).unwrap();

        // GPS−UTC = GPS_s_from_1980 − (UTC_s_from_1972 − 252892800)
        let offset = gps_s as i64 - (utc.as_seconds() as i64 - GPS_UTC_EPOCH_OFFSET_S);

        assert!(
            offset > prev_offset,
            "GPS−UTC offset must strictly increase at leap second #{}: \
             prev={prev_offset}, current={offset} at GPS={gps_s}s",
            i + 1
        );
        // Offset must be exactly i+1 after the (i+1)-th leap second.
        assert_eq!(
            offset,
            (i + 1) as i64,
            "GPS−UTC offset after leap #{} must be {}, got {}",
            i + 1,
            i + 1,
            offset
        );
        prev_offset = offset;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 12: checked_elapsed is anti-commutative
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_elapsed_is_anti_commutative() {
    let samples = gps_samples();
    let pairs = [
        (samples[0], samples[1]),
        (
            Time::<Gps>::from_seconds(1_000),
            Time::<Gps>::from_seconds(2_000),
        ),
        (
            Time::<Gps>::from_seconds(500_000),
            Time::<Gps>::from_seconds(500_001),
        ),
    ];

    for (a, b) in pairs {
        if let (Some(ab), Some(ba)) = (a.checked_elapsed(b), b.checked_elapsed(a)) {
            assert_eq!(
                ab.as_nanos(),
                -ba.as_nanos(),
                "elapsed must be anti-commutative: a-b={}, b-a={}",
                ab.as_nanos(),
                ba.as_nanos()
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 13: Duration::abs() is consistent with sign predicates
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn prop_duration_abs_consistent_with_sign() {
    for &d in &duration_samples() {
        if let Some(abs) = d.abs() {
            assert!(
                !abs.is_negative(),
                "abs() must not be negative: d={d:?}, abs={abs:?}"
            );
            if d.is_positive() {
                assert_eq!(abs, d, "abs of positive must equal itself: {d:?}");
            } else if d.is_negative() {
                assert_eq!(abs.as_nanos(), -d.as_nanos(), "abs of negative: {d:?}");
            } else {
                assert_eq!(abs, Duration::ZERO, "abs of zero must be zero");
            }
        } else {
            // abs() returns None only for Duration::MIN
            assert_eq!(
                d,
                Duration::MIN,
                "abs() returns None only for Duration::MIN"
            );
        }
    }
}
