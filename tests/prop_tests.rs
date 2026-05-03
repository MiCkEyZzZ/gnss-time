//! # Property-based tests using `proptest`
//!
//! This file uses the [`proptest`] crate to generate random inputs and verify
//! mathematical invariants of `gnss-time`.
//!
//! ## Why `#![cfg(feature = "std")]`?
//!
//! `proptest` requires `std` (it uses `std::collections`, thread-local RNG,
//! and the `std::io` trait). The `gnss-time` crate is `#![no_std]` by default,
//! so `proptest` can only run when the consumer enables the `std` feature.
//!
//! Build matrix:
//!
//! | Command | Runs these tests? |
//! |---------|-------------------|
//! | `cargo test` (host, default) | ✅ yes — `std` is implied on host |
//! | `cargo test --features std` | ✅ yes |
//! | `cargo test --no-default-features` | ❌ no — proptest requires std |
//! | `cargo check --target thumbv7em-none-eabihf` | ❌ n/a — integration tests not built for bare-metal |
//!
//! Deterministic coverage that always runs is in `tests/prop_deterministic.rs`.

// Guard the entire file: compile only when std is available.
// On a host `cargo test` run, `std` is always available even without
// `--features std`, because the test harness itself links std.
// The cfg guard here makes it explicit and prevents confusion.
#![cfg(feature = "std")]

use gnss_time::{
    convert::{ConvertResult, IntoScale, IntoScaleWith},
    gps_to_utc,
    scale::{Beidou, Galileo, Gps, Tai, Utc},
    utc_to_gps, Duration, LeapSeconds, Time,
};
use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Strategies
// ─────────────────────────────────────────────────────────────────────────────

/// GPS timestamps in the useful range: GPS epoch to ~2100.
/// Upper bound chosen so that GPS + 19 s never overflows u64.
fn gps_strategy() -> impl Strategy<Value = Time<Gps>> {
    // ~2100: GPS week 6200, ~3.8 × 10¹⁸ ns
    (0u64..3_800_000_000_000_000_000).prop_map(Time::<Gps>::from_nanos)
}

/// GPS timestamps in a narrow range around a specific leap-second boundary.
/// Used to stress-test the boundary detection logic.
fn gps_near_leap(boundary_s: u64) -> impl Strategy<Value = Time<Gps>> {
    let lo = boundary_s.saturating_sub(3) * 1_000_000_000;
    let hi = (boundary_s + 3) * 1_000_000_000;
    (lo..=hi).prop_map(Time::<Gps>::from_nanos)
}

/// Signed nanosecond durations in the safe range (no overflow in tests).
fn duration_strategy() -> impl Strategy<Value = Duration> {
    (-1_000_000_000_000_000_000i64..=1_000_000_000_000_000_000i64).prop_map(Duration::from_nanos)
}

/// Two durations, both fitting comfortably in i64 so their sum doesn't
/// overflow — lets commutativity / associativity tests always succeed.
fn small_duration_strategy() -> impl Strategy<Value = Duration> {
    (-1_000_000_000_000i64..=1_000_000_000_000i64).prop_map(Duration::from_nanos)
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 1: GPS → TAI → GPS = t
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_gps_tai_gps_roundtrip(nanos in 0u64..3_800_000_000_000_000_000) {
        let t = Time::<Gps>::from_nanos(nanos);
        let tai: Time<Tai> = t.into_scale().unwrap();
        let back: Time<Gps> = tai.into_scale().unwrap();

        prop_assert_eq!(t, back);
        // TAI offset invariant
        prop_assert_eq!(tai.as_nanos(), nanos + 19_000_000_000);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 2: GPS → Galileo → GPS = t  (identity scale)
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_gps_galileo_gps_roundtrip(t in gps_strategy()) {
        let gal: Time<Galileo> = t.into_scale().unwrap();
        let back: Time<Gps> = gal.into_scale().unwrap();

        prop_assert_eq!(t, back);
        // Identity invariant: same nanoseconds
        prop_assert_eq!(t.as_nanos(), gal.as_nanos());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 3: GPS → BeiDou → GPS = t
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_gps_beidou_gps_roundtrip(
        // BDT = GPS − 14 s; require GPS ≥ 14 s to avoid underflow
        nanos in 14_000_000_001u64..3_800_000_000_000_000_000
    ) {
        let t = Time::<Gps>::from_nanos(nanos);
        let bdt: Time<Beidou> = t.into_scale().unwrap();
        let back: Time<Gps> = bdt.into_scale().unwrap();

        prop_assert_eq!(t, back);
        // Offset invariant: BDT is exactly 14 seconds behind GPS
        prop_assert_eq!(bdt.as_nanos() + 14_000_000_000, nanos);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 4: GPS → UTC → GPS = t  (outside ambiguous window)
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_gps_utc_gps_roundtrip_exact(t in gps_strategy()) {
        let ls = LeapSeconds::builtin();

        let result: ConvertResult<Time<Utc>> = t.into_scale_with_checked(ls).unwrap();

        match result {
            ConvertResult::Exact(utc) => {
                let back = utc_to_gps(utc, ls).unwrap();
                prop_assert_eq!(t, back,
                    "GPS→UTC→GPS roundtrip failed for t={} ns", t.as_nanos());
            }
            ConvertResult::AmbiguousLeapSecond(_) => {
                // Skip — ambiguous points are covered by separate properties.
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 5: Duration — commutativity of addition
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_duration_add_commutative(
        a in small_duration_strategy(),
        b in small_duration_strategy()
    ) {
        // Small range guarantees no overflow → always Some
        let ab = a.checked_add(b).unwrap();
        let ba = b.checked_add(a).unwrap();
        prop_assert_eq!(ab, ba);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 6: Duration — associativity of addition
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_duration_add_associative(
        a in small_duration_strategy(),
        b in small_duration_strategy(),
        c in small_duration_strategy()
    ) {
        let ab_c = a.checked_add(b).and_then(|v| v.checked_add(c));
        let a_bc = b.checked_add(c).and_then(|v| a.checked_add(v));

        match (ab_c, a_bc) {
            (Some(l), Some(r)) => prop_assert_eq!(l, r),
            (None, None) => {} // consistent overflow
            _ => {} // boundary: acceptable for extreme values
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 7: Duration — zero is additive identity
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_duration_zero_identity(d in duration_strategy()) {
        prop_assert_eq!(d.checked_add(Duration::ZERO), Some(d));
        prop_assert_eq!(Duration::ZERO.checked_add(d), Some(d));
        prop_assert_eq!(d.checked_sub(Duration::ZERO), Some(d));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 8: Duration — double negation is identity
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_duration_double_negation(
        // Exclude MIN: no positive counterpart in i64
        nanos in (i64::MIN + 1)..=i64::MAX
    ) {
        let d = Duration::from_nanos(nanos);
        prop_assert_eq!(-(-d), d);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 9: Time<S> + d − d == Time<S>
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_time_add_sub_inverse(
        nanos in 1_000_000_000_000u64..3_000_000_000_000_000_000,
        delta in -1_000_000_000_000i64..=1_000_000_000_000
    ) {
        let t = Time::<Gps>::from_nanos(nanos);
        let d = Duration::from_nanos(delta);

        if let Some(t_plus_d) = t.checked_add(d) {
            if let Some(back) = t_plus_d.checked_sub_duration(d) {
                prop_assert_eq!(t, back);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 10: Ord<Time<Gps>> is consistent with u64 ordering
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_time_ord_consistent_with_u64(a in gps_strategy(), b in gps_strategy()) {
        let time_cmp = a.cmp(&b);
        let u64_cmp = a.as_nanos().cmp(&b.as_nanos());
        prop_assert_eq!(time_cmp, u64_cmp);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 11: GPS → UTC is monotone (random pairs in the same stable interval)
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_gps_utc_monotone_post_2017(
        a_s in 1_167_350_419u64..2_000_000_000,
        b_s in 1_167_350_419u64..2_000_000_000
    ) {
        let ls = LeapSeconds::builtin();
        let gps_a = Time::<Gps>::from_seconds(a_s);
        let gps_b = Time::<Gps>::from_seconds(b_s);

        let utc_a = gps_to_utc(gps_a, ls).unwrap();
        let utc_b = gps_to_utc(gps_b, ls).unwrap();

        let gps_order = gps_a.cmp(&gps_b);
        let utc_order = utc_a.cmp(&utc_b);

        prop_assert_eq!(
            gps_order,
            utc_order,
            "GPS ordering must be preserved in UTC: a={}, b={}",
            a_s,
            b_s
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 12: AmbiguousLeapSecond only near known boundaries
// ─────────────────────────────────────────────────────────────────────────────

/// The 18 GPS boundary seconds (the second where AmbiguousLeapSecond occurs).
const BOUNDARY_SECONDS: &[u64] = &[
    46_828_800,
    78_364_801,
    109_900_802,
    173_059_203,
    252_028_804,
    315_187_205,
    346_723_206,
    393_984_007,
    425_520_008,
    457_056_009,
    504_489_610,
    551_750_411,
    599_184_012,
    820_108_813,
    914_803_214,
    1_025_136_015,
    1_119_744_016,
    1_167_264_017,
];

/// Returns true if `gps_s` is within 1 second of any known leap boundary.
fn near_any_boundary(gps_s: u64) -> bool {
    BOUNDARY_SECONDS.iter().any(|&b| gps_s.abs_diff(b) <= 1)
}

proptest! {
    /// Any GPS time NOT within 1 second of a known boundary must yield Exact.
    #[test]
    fn prop_ambiguous_only_near_boundaries(t in gps_strategy()) {
        let ls = LeapSeconds::builtin();
        let gps_s = t.as_nanos() / 1_000_000_000;

        if near_any_boundary(gps_s) {
            // Skip — boundary neighbourhood is tested elsewhere.
            return Ok(());
        }

        let result: ConvertResult<Time<Utc>> = t.into_scale_with_checked(ls).unwrap();

        prop_assert!(
            result.is_exact(),
            "GPS time outside leap window (GPS={} s) must be Exact, got Ambiguous",
            gps_s
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 13: near leap boundaries, stress-test the 1-second window
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_gps_near_leap_converts_consistently(
        t in gps_near_leap(1_167_264_017)
    ) {
        let ls = LeapSeconds::builtin();
        let result: ConvertResult<Time<Utc>> = t.into_scale_with_checked(ls).unwrap();

        match result {
            ConvertResult::Exact(utc) => {
                let back = utc_to_gps(utc, ls).unwrap();
                prop_assert_eq!(back, t);
            }
            ConvertResult::AmbiguousLeapSecond(_utc) => {
                // допустимый результат для точки около leap-second границы
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 14: Duration::abs is consistent with sign
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_duration_abs_sign_consistent(
        nanos in (i64::MIN + 1)..=i64::MAX
    ) {
        let d = Duration::from_nanos(nanos);
        let abs = d.abs().unwrap(); // safe: MIN excluded

        prop_assert!(!abs.is_negative());

        if d.is_positive() {
            prop_assert_eq!(abs, d);
        } else if d.is_negative() {
            prop_assert_eq!(abs.as_nanos(), -nanos);
        } else {
            prop_assert_eq!(abs, Duration::ZERO);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 15: checked_elapsed anti-commutativity
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_elapsed_anti_commutative(a in gps_strategy(), b in gps_strategy()) {
        if let (Some(ab), Some(ba)) = (a.checked_elapsed(b), b.checked_elapsed(a)) {
            prop_assert_eq!(
                ab.as_nanos(),
                -ba.as_nanos(),
                "elapsed anti-commutativity: a-b={} b-a={}",
                ab.as_nanos(),
                ba.as_nanos()
            );
        }
    }
}
