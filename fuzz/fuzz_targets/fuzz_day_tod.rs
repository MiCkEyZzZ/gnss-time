#![no_main]

//! Fuzz target: `Time::<Glonass>::from_day_tod` constructor — validation,
//! error classification, exactness and field roundtrip.
//!
//! Mirror of `fuzz_week_tow` with the GLONASS day/TOD edge set. Two input
//! modes, selected by the high bit of `data[0]`:
//!
//! - **RAW** (`data[0] & 0x80 == 0`, `len >= 17`): bytes 1..5 are a `u32`
//!   GLONASS day number, bytes 5..13 a `u64` TOD-whole-seconds, bytes 13..17 a
//!   `u32` sub-second nanos — the full constructor domain. Verifies "no panic /
//!   no unexpected error" plus exact error classification over the entire `day
//!   × tod` space, including the extreme high end near `u64::MAX`.
//! - **BOUNDARY** (`data[0] & 0x80 != 0`, `len >= 16`): `data[1..4]` select one
//!   edge from each of the three constructor edge sets (day, TOD seconds,
//!   sub-second nanos); bytes 4..16 carry `i32` jitters folded into small
//!   windows around the chosen edges. This guarantees that **every**
//!   constructor boundary is exercised on every run:
//!   - the TOD-seconds validity edge (`86_400`);
//!   - the sub-second-nanos validity edge (`1_000_000_000`);
//!   - the `u64`-storage overflow rim (`day ≈ 213_503` with a non-zero TOD);
//!   - the four-day-rollover / historic counter edges (`1_461`, `65_535`,
//!     `262_143`, ...);
//!   - `day_of_week()` stays in `1..=7` and `is_weekend()` agrees with it.
//!
//! Invariants (see `docs/INVARIANTS.md`):
//!
//! 1. `from_day_tod` never panics. The only legal results are exactly:
//!    - [`GnssTimeError::InvalidInput`] iff `tod.seconds >= 86_400` or
//!      `tod.nanos >= 1_000_000_000` (documented constructor contract);
//!    - [`GnssTimeError::Overflow`] iff the TOD is valid but `day *
//!      86_400_000_000_000 + tod_ns` exceeds `u64::MAX`;
//!    - `Ok` otherwise.
//!    Any other error / `Ok` mix is a genuine defect.
//! 2. On `Ok`, the constructor is **exact**, not approximate:
//!    - `as_nanos() == day * DAY_NS + tod_ns`;
//!    - the field accessors roundtrip: `day()`, `tod_seconds()`,
//!      `sub_second_nanos()` reproduce the input fields bit-for-bit.
//! 3. Determinism: the constructor is a pure function of its inputs.
//! 4. `day_of_week()` returns a value in `1..=7` and `is_weekend()` agrees
//!    with it on every representable day. (Exact weekdays are pinned by the
//!    unit tests for known dates, not re-derived here.)
//!
//! ## Run
//!
//! ```sh
//! cargo fuzz run fuzz_day_tod -- -max_total_time=300 -max_len=17
//! cargo fuzz run fuzz_day_tod -- -max_total_time=300 -max_len=17 \
//!     -dict=fuzz_day_tod.dict
//! ```
//!
//! Prime the corpus first with `tools/gen_corpus.sh`.

use gnss_time::{DurationParts, Glonass, GnssTimeError, Time};
use libfuzzer_sys::fuzz_target;

/// Nanoseconds per GLONASS day (86_400 s).
const DAY_NS: u64 = 86_400_000_000_000;

/// TOD whole-seconds validity boundary (exclusive).
const SECONDS_PER_DAY: u64 = 86_400;

/// Sub-second nanos validity boundary (exclusive).
const NANOS_PER_SECOND: u64 = 1_000_000_000;

// ── Edge sets ────────────────────────────────────────────────────────────────

/// Day edges: epoch, the 4-year GLONASS cycle (1461 days), historic counter
/// widths, the overflow rim and extremes. `day <= 213_503` is representable
/// (with a bounded TOD); `>= 213_504` always overflows.
const DAY_EDGES: [u32; 16] = [
    0, // GLONASS epoch
    1,
    1_460, // day 0..=1460 spans a full 4-year interval
    1_461, // next 4-year interval starts
    1_462,
    10_000,
    100_000,
    209_715, // 0x33333 — well beyond any historic counter, still in range
    213_502,
    213_503, // last representable day (with bounded TOD)
    213_504, // overflow rim
    213_505,
    65_535,  // u16::MAX — historic day counter width
    262_143, // 2^18 − 1 — MCS counter width
    1_000_000,
    u32::MAX,
];

/// TOD whole-seconds edges around the `86_400` validity boundary.
const TOD_EDGES: [u64; 8] = [
    0,
    1,
    1_439,  // minute-23:59 boundary space
    86_399, // last valid second of the day
    86_400, // first invalid
    86_401,
    u32::MAX as u64,
    u64::MAX,
];

/// Sub-second nanos edges around the `1_000_000_000` validity boundary.
const NANOS_EDGES: [u32; 8] = [
    0,
    1,
    999_999_998,
    999_999_999,   // last valid sub-second value
    1_000_000_000, // first invalid
    1_000_000_001,
    u32::MAX - 1,
    u32::MAX,
];

/// Clamp the documented overflow rim to reality. These two asserts pin the
/// day-overflow constants so a typo in `DAY_NS` breaks the build, not the
/// fuzzer's expectations.
const _: () = {
    assert!((213_503u128 * DAY_NS as u128) < (u64::MAX as u128));
    assert!((u64::MAX as u128) < (213_504u128 * DAY_NS as u128));
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Clamp `base + jitter` into `[0, u64::MAX]` instead of wrapping, so an edge
/// plus jitter never silently wraps into a different validity class.
#[inline]
fn sat_add(
    base: u64,
    jitter: i32,
) -> u64 {
    if jitter >= 0 {
        base.saturating_add(jitter as u64)
    } else {
        base.saturating_sub(jitter.unsigned_abs() as u64)
    }
}

/// Fold any bit pattern into `[-W/2, +W/2)` so small byte mutations produce
/// small numeric steps that walk the fuzzer across boundaries.
#[inline]
fn fold(
    jitter: i64,
    width: i64,
) -> i32 {
    (jitter % width - width / 2) as i32
}

/// Run the full invariant battery for one `(day, secs, nanos)` triple.
fn check_constructor(
    day: u32,
    secs: u64,
    nanos: u32,
) {
    let tod = DurationParts {
        seconds: secs,
        nanos,
    };

    // Expected classification straight from the documented contract.
    let tod_bad = secs >= SECONDS_PER_DAY || u64::from(nanos) >= NANOS_PER_SECOND;
    let expected = if tod_bad {
        ExpectedKind::Invalid
    } else {
        // Exact u128 arithmetic — no overflow possibility, so this never
        // wraps and gives the ground truth for the Overflow classification.
        let day_ns = u128::from(day) * u128::from(DAY_NS);
        let tod_ns = u128::from(secs) * u128::from(NANOS_PER_SECOND) + u128::from(nanos);
        if day_ns + tod_ns > u128::from(u64::MAX) {
            ExpectedKind::Overflow
        } else {
            ExpectedKind::Ok
        }
    };

    let res = Time::<Glonass>::from_day_tod(day, tod);
    match (&res, &expected) {
        (Ok(t), ExpectedKind::Ok) => {
            assert_eq!(
                t.as_nanos(),
                u64::from(day) * DAY_NS + secs * NANOS_PER_SECOND + u64::from(nanos),
                "constructor not exact at day={day}, secs={secs}, nanos={nanos}"
            );
            assert_eq!(
                t.day(),
                day,
                "day() roundtrip at day={day}, secs={secs}, nanos={nanos}"
            );
            assert_eq!(
                t.tod_seconds(),
                secs as u32,
                "tod_seconds() roundtrip at day={day}, secs={secs}, nanos={nanos}"
            );
            assert_eq!(
                t.sub_second_nanos(),
                nanos,
                "sub_second_nanos() roundtrip at day={day}, secs={secs}, nanos={nanos}"
            );
            // day_of_week() must always stay in the ISO range 1..=7; the
            // exact weekday is pinned by the unit tests for known dates.
            let dow = t.day_of_week();
            assert!(
                (1..=7).contains(&dow),
                "day_of_week() out of range at day={day}: {dow}"
            );
            assert_eq!(
                t.is_weekend(),
                dow == 6 || dow == 7,
                "is_weekend() mismatch at day={day}: {}, dow={dow}",
                t.is_weekend()
            );
        }
        (Err(GnssTimeError::InvalidInput(_)), ExpectedKind::Invalid) => {}
        (Err(GnssTimeError::Overflow), ExpectedKind::Overflow) => {}
        (Err(e), ExpectedKind::Invalid) => panic!(
            "expected InvalidInput but got another error at day={day}, secs={secs}, nanos={nanos}: {e:?}"
        ),
        (Err(e), ExpectedKind::Overflow) => panic!(
            "expected Overflow but got another error at day={day}, secs={secs}, nanos={nanos}: {e:?}"
        ),
        (Err(e), ExpectedKind::Ok) => panic!(
            "constructor rejected valid input at day={day}, secs={secs}, nanos={nanos}: {e:?}"
        ),
        (Ok(_), ExpectedKind::Invalid) => panic!(
            "constructor accepted invalid input at day={day}, secs={secs}, nanos={nanos}"
        ),
        (Ok(_), ExpectedKind::Overflow) => panic!(
            "constructor accepted overflowing input at day={day}, secs={secs}, nanos={nanos}"
        ),
    };

    // Determinism: decoding the same triple twice yields identical results.
    let again = Time::<Glonass>::from_day_tod(day, tod);
    match (res, again) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(a), Err(b)) => assert_eq!(
            core::mem::discriminant(&a),
            core::mem::discriminant(&b),
            "nondeterministic error at day={day}, secs={secs}, nanos={nanos}"
        ),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
            panic!("nondeterministic result at day={day}, secs={secs}, nanos={nanos}")
        }
    }
}

// ── Expected classification ──────────────────────────────────────────────────

/// The constructor's documented outcome for a given `(day, secs, nanos)`
/// triple, independent of the concrete `GnssTimeError` payload.
enum ExpectedKind {
    Ok,
    Invalid,
    Overflow,
}

fuzz_target!(|data: &[u8]| {
    let Some(&mode) = data.first() else {
        return; // libFuzzer feeds the empty input first
    };

    // RAW mode: the full u32/u64 constructor domain.
    if mode & 0x80 == 0 {
        if data.len() < 17 {
            return;
        }
        let day = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let secs = u64::from_le_bytes([
            data[5], data[6], data[7], data[8], data[9], data[10], data[11], data[12],
        ]);
        let nanos = u32::from_le_bytes([data[13], data[14], data[15], data[16]]);
        check_constructor(day, secs, nanos);
        return;
    }

    // BOUNDARY mode: structured exploration around every constructor edge.
    if data.len() < 16 {
        return;
    }

    let day = sat_add(
        u64::from(DAY_EDGES[data[1] as usize % DAY_EDGES.len()]),
        fold(
            i64::from(i32::from_le_bytes([data[4], data[5], data[6], data[7]])),
            8,
        ),
    );
    let secs = sat_add(
        TOD_EDGES[data[2] as usize % TOD_EDGES.len()],
        fold(
            i64::from(i32::from_le_bytes([data[8], data[9], data[10], data[11]])),
            64,
        ),
    );
    let nanos = sat_add(
        u64::from(NANOS_EDGES[data[3] as usize % NANOS_EDGES.len()]),
        fold(
            i64::from(i32::from_le_bytes([data[12], data[13], data[14], data[15]])),
            4_000_000_000,
        ),
    );

    check_constructor(day as u32, secs, nanos as u32);
});
