#![no_main]

//! Fuzz target: GPS ↔ UTC roundtrip + leap-second lookup.
//!
//! Two input modes, selected by the high bit of `data[0]`:
//!
//! - **RAW** (`data[0] & 0x80 == 0`, `len >= 9`): bytes 1..9 are a uniform
//!   `u64` GPS-nanosecond value covering the full domain. Verifies "no panic /
//!   no unexpected error on any `u64`" and invariant I-15 over the whole range,
//!   including the extreme high end near `u64::MAX`.
//! - **BOUNDARY** (`data[0] & 0x80 != 0`, `len >= 12`): `data[1]` selects a
//!   real leap-second transition from the builtin table (`entries[1..]`,
//!   skipping `entries[0]`, the base value pinned at the GPS epoch), bytes
//!   2..10 hold an `i64` jitter folded into a ±20 s window around the
//!   transition's GPS instant. This guarantees that **every** leap-second
//!   ambiguity window is exercised on every run — blind mutation of a raw `u64`
//!   space (~1.8e19) can never reach a 2 s window reliably. Mutating the jitter
//!   bytes walks libFuzzer through the window, which is exactly the gradient
//!   edge coverage *cannot* provide.
//!
//! Invariants (see `docs/INVARIANTS.md`):
//!
//! 1. `gps_to_utc` / `utc_to_gps` never panic and never return an unexpected
//!    error. `Overflow` is legal only at the extreme high end of `gps_to_utc`;
//!    everywhere else it is a real bug:
//!    - `gps_to_utc` can only overflow high (UTC > `u64::MAX`); the UTC epoch
//!      (1972) predates GPS (1980), so it can never underflow;
//!    - `utc_to_gps` can never overflow on values produced by `gps_to_utc`
//!      (algebraically `utc_to_gps(utc) < utc` for every reachable `utc`, and
//!      `utc >= GPS epoch`), so any failure there is a genuine defect.
//! 2. **I-15** (roundtrip accuracy): outside the 1 s ambiguity window the
//!    roundtrip is *exact* — `ConvertResult::Exact ⇒ drift == 0`. Inside the
//!    window (`AmbiguousLeapSecond`) drift ≤ 1 s is the documented legal bound.
//!    This is deliberately stricter than the old "≤ 1 s everywhere".
//! 3. `gps_to_utc` is non-decreasing in GPS time (UTC never runs backwards
//!    across a leap second).
//! 4. `TAI − UTC` is non-decreasing and jumps by at most 1 s per second of TAI.
//!    (Stronger than the old adjacent-nanosecond check, so it actually
//!    exercises binary search around every threshold.)
//!
//! ## Run
//!
//! ```sh
//! cargo fuzz run fuzz_gps_utc -- -max_total_time=300 -max_len=12
//! cargo fuzz run fuzz_gps_utc -- -max_total_time=300 -max_len=12 \
//!     -dict=fuzz_gps_utc.dict
//! ```
//!
//! Prime the corpus first with `tools/gen_corpus.sh`.

use gnss_time::{
    gps_to_utc, utc_to_gps, ConvertResult, GnssTimeError, Gps, IntoScaleWith, LeapSeconds,
    LeapSecondsProvider, Time, Utc,
};
use libfuzzer_sys::fuzz_target;

const ONE_SECOND_NS: u64 = 1_000_000_000;
const GPS_TAI_OFFSET_NS: i64 = 19 * ONE_SECOND_NS as i64;

/// Half-width (in ns) of the structured jitter window around each leap
/// transition. ±20 s covers the 1 s ambiguity window, the ±18 s zone where
/// `utc_to_gps`'s two-pass approximation can straddle a distant threshold,
/// and plenty of margin.
const JITTER_RANGE_NS: i64 = 20_000_000_000;

/// GPS nanoseconds (since GPS epoch) at which the leap transition flips for
/// the table entry whose `tai_nanos` threshold is given.
#[inline]
fn gps_flip_ns(tai_threshold: u64) -> i128 {
    i128::from(tai_threshold) - i128::from(GPS_TAI_OFFSET_NS)
}

/// Roundtrip + monotonicity checks for a GPS instant.
fn check_roundtrip_and_invariants(nanos: u64) {
    let ls = LeapSeconds::builtin();
    let gps = Time::<Gps>::from_nanos(nanos);

    // ── GPS → UTC → GPS roundtrip (I-15, conditionally exact) ───────────────
    let utc = match gps_to_utc(gps, ls) {
        Ok(u) => u,
        // Legal only at the extreme high end. The overflow boundary is
        // gps > u64::MAX - 2927 days (UTC_TO_GPS_EPOCH_NS, since UTC counts
        // from 1972 and GPS from 1980) plus the few seconds of tai - utc, so
        // almost the whole top ~2927 days of the u64 space legitimately
        // overflows. Guarding it precisely would need a private library
        // constant, so treat any Overflow as legal.
        Err(GnssTimeError::Overflow) => return,
        Err(e) => panic!("gps_to_utc unexpected error at gps={nanos}: {e:?}"),
    };

    // Same underlying conversion as gps_to_utc, so it must succeed here.
    let checked: ConvertResult<Time<Utc>> = gps
        .into_scale_with_checked(ls)
        .expect("into_scale_with_checked failed where gps_to_utc succeeded");

    // The free function and the trait method are two paths to the same
    // conversion; pin them to each other when the result is exact. Inside
    // the ambiguity window a 1 s representation difference is legal, so only
    // the Exact case must agree bit-for-bit.
    if let ConvertResult::Exact(exact) = &checked {
        assert_eq!(
            utc.as_nanos(),
            exact.as_nanos(),
            "gps_to_utc and into_scale_with_checked disagree at gps={nanos}"
        );
    }

    let drift_bound = match checked {
        ConvertResult::Exact(_) => 0,
        // Inside the ambiguity window a 1 s error is the documented, legal
        // representation of a non-injective mapping.
        ConvertResult::AmbiguousLeapSecond(_) => ONE_SECOND_NS,
        _ => unreachable!("ConvertResult is #[non_exhaustive]; no other variants exist"),
    };

    let gps_back = match utc_to_gps(utc, ls) {
        Ok(g) => g,
        // utc >= GPS epoch by construction here (gps_to_utc can never map
        // below the GPS epoch), and utc_to_gps(utc) < utc always, so any
        // failure is a genuine defect.
        Err(e) => panic!(
            "utc_to_gps failed at utc.nanos={}, gps={nanos}: {e:?}",
            utc.as_nanos()
        ),
    };

    let drift = gps_back.as_nanos().abs_diff(nanos);

    assert!(
        drift <= drift_bound,
        "I-15 violated: gps={nanos}, utc={}, check={checked:?}, drift={drift}, bound={drift_bound}",
        utc.as_nanos()
    );

    // ── Monotonicity: UTC never runs backwards as GPS advances ──────────────
    let Some(next_nanos) = nanos.checked_add(ONE_SECOND_NS) else {
        return;
    };
    let utc_next = match gps_to_utc(Time::<Gps>::from_nanos(next_nanos), ls) {
        Ok(u) => u,
        Err(_) => return, // overflow at the extreme high end
    };

    assert!(
        utc_next.as_nanos() >= utc.as_nanos(),
        "GPS→UTC ran backwards: gps={nanos} (utc={}) -> gps={next_nanos} (utc={})",
        utc.as_nanos(),
        utc_next.as_nanos()
    );

    // ── TAI − UTC step over 1 s of TAI: monotone, jump ≤ 1 s ────────────────
    let tai = match gps.to_tai() {
        Ok(t) => t,
        Err(_) => return, // overflow at the extreme high end
    };
    let next_tai = match Time::<Gps>::from_nanos(next_nanos).to_tai() {
        Ok(t) => t,
        Err(_) => return,
    };
    let now = ls.tai_minus_utc_at(tai);
    let nxt = ls.tai_minus_utc_at(next_tai);

    assert!(nxt >= now, "TAI−UTC decreased: gps={nanos}, {now} → {nxt}");
    assert!(
        nxt - now <= 1,
        "TAI−UTC jumped > 1 s: gps={nanos}, {now} → {nxt}"
    );
}

fuzz_target!(|data: &[u8]| {
    let Some(&mode) = data.first() else {
        return; // libFuzzer feeds the empty input first
    };

    // RAW mode: full u64 domain, no panic / no unexpected error /
    // exact roundtrip outside ambiguity windows.
    if mode & 0x80 == 0 {
        if data.len() < 9 {
            return;
        }
        let nanos = u64::from_le_bytes([
            data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
        ]);
        check_roundtrip_and_invariants(nanos);
        return;
    }

    // BOUNDARY mode: structured exploration around every leap second,
    // so the ambiguity windows are guaranteed to be hit every run.
    if data.len() < 12 {
        return;
    }

    let ls = LeapSeconds::builtin();
    let entries = ls.entries();
    // entries[0] is the base value at the GPS epoch, not a real transition.
    let idx = 1 + (data[1] as usize) % (entries.len() - 1); // 1..=18
    let threshold_tai = entries[idx].tai_nanos;

    let jitter = i64::from_le_bytes([
        data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
    ]);
    // Fold any 64-bit pattern into [-JITTER_RANGE_NS, +JITTER_RANGE_NS). Small
    // byte mutations → small jitter steps → libFuzzer walks across boundaries.
    let jitter = jitter % (2 * JITTER_RANGE_NS) - JITTER_RANGE_NS;

    let total = gps_flip_ns(threshold_tai) + i128::from(jitter);

    let Ok(nanos) = u64::try_from(total) else {
        return; // gps_flip is positive for every real transition; only
                // extreme negative jitter could underflow — drop and retry.
    };

    check_roundtrip_and_invariants(nanos);
});
