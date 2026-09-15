#![no_main]

//! Fuzz target: UTC ↔ GPS roundtrip + leap-second lookup (UTC side).
//!
//! Mirror of `fuzz_gps_utc` with the input expressed in the UTC domain,
//! exercising `utc_to_gps` on the UTC side.
//!
//! Two input modes, selected by the high bit of `data[0]`:
//!
//! - **RAW** (`data[0] & 0x80 == 0`, `len >= 9`): bytes 1..9 are a uniform
//!   `u64` UTC-nanosecond value since 1972-01-01, covering the full domain.
//! - **BOUNDARY** (`data[0] & 0x80 != 0`, `len >= 12`): `data[1]` selects a
//!   real leap-second transition from the builtin table (`entries[1..]`,
//!   skipping `entries[0]`, the base value pinned at the GPS epoch), bytes
//!   2..10 hold an `i64` jitter folded into a ±20 s window around the
//!   transition's UTC instant, so every ambiguity window is hit every run.
//!
//! Invariants (see `docs/INVARIANTS.md`):
//! 1. `utc_to_gps` never panics and never returns an unexpected error.
//!    `Overflow` is legal only when UTC precedes the GPS epoch (1980-01-06),
//!    i.e. `utc_ns < UTC_TO_GPS_EPOCH_NS`, so any other failure is a defect;
//!    `utc_to_gps` can never overflow high (`gps_ns < utc_ns` always).
//! 2. **I-12** (roundtrip accuracy): outside the 1 s ambiguity window the
//!    roundtrip is exact — `ConvertResult::Exact ⇒ drift == 0`; inside it drift
//!    ≤ 1 s is the legal bound.
//! 3. `utc_to_gps` is non-decreasing in UTC (GPS never runs backwards when
//!    encoded the other way).
//! 4. `TAI − UTC` is non-decreasing and jumps by at most 1 s per second.
//!
//! ## Run
//!
//! ```sh
//! cargo fuzz run fuzz_utc_to_gps -- -max_total_time=300 -max_len=12 \
//!     -dict=fuzz_utc_to_gps.dict
//! ```

use gnss_time::{
    gps_to_utc, utc_to_gps, ConvertResult, GnssTimeError, Gps, IntoScaleWith, LeapSeconds,
    LeapSecondsProvider, Time, Utc,
};
use libfuzzer_sys::fuzz_target;

const ONE_SECOND_NS: u64 = 1_000_000_000;

/// UTC nanos (since 1972-01-01) at the GPS epoch 1980-01-06.
const UTC_TO_GPS_EPOCH_NS: u64 = 252_892_800_000_000_000;

/// Half-width of the structured jitter window around each leap transition.
const JITTER_RANGE_NS: i64 = 20_000_000_000;

/// UTC nanoseconds at which the leap transition flips for the table entry
/// whose `tai_nanos` threshold is given (`TAI − UTC = offset` after the flip).
#[inline]
fn utc_flip_ns(
    tai_threshold: u64,
    offset: i32,
) -> i128 {
    i128::from(tai_threshold) - i128::from(i64::from(offset) * ONE_SECOND_NS as i64)
}

/// Roundtrip + monotonicity checks for a UTC instant.
fn check_roundtrip_and_invariants(nanos: u64) {
    let ls = LeapSeconds::builtin();
    let utc = Time::<Utc>::from_nanos(nanos);

    // ── UTC → GPS → UTC roundtrip (I-12, conditionally exact) ───────────────
    let gps = match utc_to_gps(utc, ls) {
        Ok(g) => g,
        // Legal only before the GPS epoch (UTC runs earlier than GPS can
        // represent), i.e. below the epoch boundary.
        Err(GnssTimeError::Overflow) => {
            assert!(
                nanos < UTC_TO_GPS_EPOCH_NS,
                "utc_to_gps overflowed above the GPS epoch boundary: utc={nanos}"
            );
            return;
        }
        Err(e) => panic!("utc_to_gps unexpected error at utc={nanos}: {e:?}"),
    };

    // Same underlying conversion as utc_to_gps, so it must succeed here.
    let checked: ConvertResult<Time<Gps>> = utc
        .into_scale_with_checked(ls)
        .expect("into_scale_with_checked failed where utc_to_gps succeeded");

    // Pin the two API paths to each other on the Exact branch; inside the
    // ambiguity window the two representations may legitimately differ by 1 s.
    if let ConvertResult::Exact(ref exact) = checked {
        assert_eq!(
            gps.as_nanos(),
            exact.as_nanos(),
            "utc_to_gps and into_scale_with_checked disagree at utc={nanos}"
        );
    }

    let drift_bound = match checked {
        ConvertResult::Exact(_) => 0,
        // Inside the ambiguity window a 1 s error is the documented, legal
        // representation of a non-injective mapping.
        ConvertResult::AmbiguousLeapSecond(_) => ONE_SECOND_NS,
    };

    let utc_back = match gps_to_utc(gps, ls) {
        Ok(u) => u,
        Err(e) => panic!(
            "gps_to_utc failed at gps={}, utc={nanos}: {e:?}",
            gps.as_nanos()
        ),
    };

    let drift = utc_back.as_nanos().abs_diff(nanos);

    assert!(
        drift <= drift_bound,
        "I-12 violated: utc={nanos}, gps={}, check={checked:?}, drift={drift}, bound={drift_bound}",
        gps.as_nanos()
    );

    // ── Monotonicity: GPS never runs backwards as UTC advances ──────────────
    let Some(next_nanos) = nanos.checked_add(ONE_SECOND_NS) else {
        return;
    };
    let Ok(gps_next) = utc_to_gps(Time::<Utc>::from_nanos(next_nanos), ls) else {
        return; // underflow only below the epoch, already past it here
    };

    assert!(
        gps_next.as_nanos() >= gps.as_nanos(),
        "UTC→GPS ran backwards: utc={nanos} (gps={}) -> utc={next_nanos} (gps={})",
        gps.as_nanos(),
        gps_next.as_nanos()
    );

    // ── TAI − UTC step over 1 s of TAI: monotone, jump ≤ 1 s ────────────────
    let tai = match gps.to_tai() {
        Ok(t) => t,
        Err(_) => return, // overflow at the extreme high end
    };
    let next_gps_nanos = gps_next.as_nanos();
    let next_tai = match Time::<Gps>::from_nanos(next_gps_nanos).to_tai() {
        Ok(t) => t,
        Err(_) => return,
    };
    let now = ls.tai_minus_utc_at(tai);
    let nxt = ls.tai_minus_utc_at(next_tai);

    assert!(nxt >= now, "TAI−UTC decreased: utc={nanos}, {now} → {nxt}");
    assert!(
        nxt - now <= 1,
        "TAI−UTC jumped > 1 s: utc={nanos}, {now} → {nxt}"
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

    // BOUNDARY mode: structured exploration around every leap second.
    if data.len() < 12 {
        return;
    }

    let ls = LeapSeconds::builtin();
    let entries = ls.entries();
    // entries[0] is the base value at the GPS epoch, not a real transition.
    let idx = 1 + (data[1] as usize) % (entries.len() - 1);
    let entry = entries[idx];

    let jitter = i64::from_le_bytes([
        data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
    ]);
    // Fold any 64-bit pattern into [-JITTER_RANGE_NS, +JITTER_RANGE_NS). Small
    // byte mutations → small jitter steps → libFuzzer walks across boundaries.
    let jitter = jitter % (2 * JITTER_RANGE_NS) - JITTER_RANGE_NS;

    let total = utc_flip_ns(entry.tai_nanos, entry.tai_minus_utc) + i128::from(jitter);

    let Ok(nanos) = u64::try_from(total) else {
        return; // every real transition flips far above the epoch; only
                // extreme negative jitter could underflow — drop and retry.
    };

    check_roundtrip_and_invariants(nanos);
});
