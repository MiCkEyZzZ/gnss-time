#![no_main]

//! Fuzz target: `Time::<Gps>::from_week_tow` constructor — validation, error
//! classification, exactness and field roundtrip.
//!
//! Two input modes, selected by the high bit of `data[0]`:
//!
//! - **RAW** (`data[0] & 0x80 == 0`, `len >= 17`): bytes 1..5 are a `u32` GPS
//!   week number, bytes 5..13 a `u64` TOW-whole-seconds, bytes 13..17 a `u32`
//!   sub-second nanos — the full constructor domain. Verifies "no panic / no
//!   unexpected error" plus exact error classification over the entire `week ×
//!   tow` space, including the extreme high end near `u64::MAX`.
//! - **BOUNDARY** (`data[0] & 0x80 != 0`, `len >= 16`): `data[1..4]` select one
//!   edge from each of the three constructor edge sets (week, TOW seconds,
//!   sub-second nanos); bytes 4..16 carry `i32` jitters folded into small
//!   windows around the chosen edges. This guarantees that **every**
//!   constructor boundary is exercised on every run:
//!   - the TOW-seconds validity edge (`604_800`);
//!   - the sub-second-nanos validity edge (`1_000_000_000`);
//!   - the `u64`-storage overflow rim (`week ≈ 30_500` with a non-zero TOW).
//!
//! Invariants (see `docs/INVARIANTS.md`):
//!
//! 1. `from_week_tow` never panics. The only legal results are exactly:
//!    - [`GnssTimeError::InvalidInput`] iff `tow.seconds >= 604_800` or
//!      `tow.nanos >= 1_000_000_000` (documented constructor contract);
//!    - [`GnssTimeError::Overflow`] iff the TOW is valid but `week *
//!      604_800_000_000_000 + tow_ns` exceeds `u64::MAX`;
//!    - `Ok` otherwise.
//!    Any other error / `Ok` mix is a genuine defect.
//! 2. On `Ok`, the constructor is **exact**, not approximate:
//!    - `as_nanos() == week * WEEK_NS + tow_ns`;
//!    - the field accessors roundtrip: `week()`, `tow_seconds()`,
//!      `sub_second_nanos()` reproduce the input fields bit-for-bit.
//! 3. Determinism: the constructor is a pure function of its inputs.
//!
//! ## Run
//!
//! ```sh
//! cargo fuzz run fuzz_week_tow -- -max_total_time=300 -max_len=17
//! cargo fuzz run fuzz_week_tow -- -max_total_time=300 -max_len=17 \
//!     -dict=fuzz_week_tow.dict
//! ```
//!
//! Prime the corpus first with `tools/gen_corpus.sh`.

use gnss_time::{DurationParts, GnssTimeError, Gps, Time};
use libfuzzer_sys::fuzz_target;

/// Nanoseconds per GPS week (604_800 s).
const WEEK_NS: u64 = 604_800_000_000_000;

/// TOW whole-seconds validity boundary (exclusive).
const SECONDS_PER_WEEK: u64 = 604_800;

/// Sub-second nanos validity boundary (exclusive).
const NANOS_PER_SECOND: u64 = 1_000_000_000;

// ── Edge sets ────────────────────────────────────────────────────────────────

/// Week edges: epoch, historic counter-rollover points, the overflow rim and
/// extremes. `week <= 30_500` is representable (with a bounded TOW);
/// `>= 30_501` always overflows.
const WEEK_EDGES: [u32; 16] = [
    0, // GPS epoch
    1,
    1023, // 0x03FF — 10-bit week rollover
    1024, // 0x0400 — first value past it
    2047, // 0x07FF — 11-bit week rollover
    2048,
    10_000,
    30_499,
    30_500, // last representable week (with bounded TOW)
    30_501, // overflow rim
    30_502,
    65_535,  // u16::MAX — historic week counter width
    262_143, // 2^18 − 1 — MCS week counter width
    1_000_000,
    4_000_000_000, // half of u32
    u32::MAX,
];

/// TOW whole-seconds edges around the `604_800` validity boundary.
const SECS_EDGES: [u64; 8] = [
    0,
    1,
    604_799, // last valid second of the week
    604_800, // first invalid
    604_801,
    u32::MAX as u64,
    u64::MAX - 1,
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

/// The documented overflow rim, tied to `WEEK_NS`: exactly `30_501` for the
/// real week length. If `WEEK_NS` ever changes, `WEEK_EDGES`, this constant
/// and the corpus generator must be re-derived — this compile-time assert
/// guards that they stay consistent with the `u64` storage bound.
const _: () = {
    assert!((30_500u128 * WEEK_NS as u128) < (u64::MAX as u128));
    assert!((u64::MAX as u128) < (30_501u128 * WEEK_NS as u128));
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

/// Run the full invariant battery for one `(week, secs, nanos)` triple.
fn check_constructor(
    week: u32,
    secs: u64,
    nanos: u32,
) {
    let tow = DurationParts {
        seconds: secs,
        nanos,
    };

    // Expected classification straight from the documented contract.
    let tow_bad = secs >= SECONDS_PER_WEEK || u64::from(nanos) >= NANOS_PER_SECOND;

    let expected = if tow_bad {
        ExpectedKind::Invalid
    } else {
        // Exact u128 arithmetic — no overflow possibility, so this never
        // wraps and gives the ground truth for the Overflow classification.
        let week_ns = u128::from(week) * u128::from(WEEK_NS);
        let tow_ns = u128::from(secs) * u128::from(NANOS_PER_SECOND) + u128::from(nanos);

        if week_ns + tow_ns > u128::from(u64::MAX) {
            ExpectedKind::Overflow
        } else {
            ExpectedKind::Ok
        }
    };

    let res = Time::<Gps>::from_week_tow(week, tow);

    let matches = match (&res, &expected) {
        (Ok(t), ExpectedKind::Ok) => {
            assert_eq!(
                t.as_nanos(),
                u64::from(week) * WEEK_NS + secs * NANOS_PER_SECOND + u64::from(nanos),
                "constructor not exact at week={week}, secs={secs}, nanos={nanos}"
            );
            assert_eq!(
                t.week(),
                week,
                "week() roundtrip at week={week}, secs={secs}, nanos={nanos}"
            );
            assert_eq!(
                t.tow_seconds(),
                secs as u32,
                "tow_seconds() roundtrip at week={week}, secs={secs}, nanos={nanos}"
            );
            assert_eq!(
                t.sub_second_nanos(),
                nanos,
                "sub_second_nanos() roundtrip at week={week}, secs={secs}, nanos={nanos}"
            );
            true
        }
        (Err(GnssTimeError::InvalidInput(_)), ExpectedKind::Invalid) => true,
        (Err(GnssTimeError::Overflow), ExpectedKind::Overflow) => true,
        (Err(e), ExpectedKind::Invalid) => panic!(
            "expected InvalidInput but got another error at week={week}, secs={secs}, nanos={nanos}: {e:?}"
        ),
        (Err(e), ExpectedKind::Overflow) => panic!(
            "expected Overflow but got another error at week={week}, secs={secs}, nanos={nanos}: {e:?}"
        ),
        (Err(e), ExpectedKind::Ok) => panic!(
            "constructor rejected valid input at week={week}, secs={secs}, nanos={nanos}: {e:?}"
        ),
        (Ok(_), ExpectedKind::Invalid) => panic!(
            "constructor accepted invalid input at week={week}, secs={secs}, nanos={nanos}"
        ),
        (Ok(_), ExpectedKind::Overflow) => panic!(
            "constructor accepted overflowing input at week={week}, secs={secs}, nanos={nanos}"
        ),
    };
    assert!(
        matches,
        "misclassified at week={week}, secs={secs}, nanos={nanos}: {res:?} (expected {expected:?})"
    );

    // Determinism: decoding the same triple twice yields identical results.
    let again = Time::<Gps>::from_week_tow(week, tow);

    match (res, again) {
        (Ok(a), Ok(b)) => assert_eq!(a, b),
        (Err(a), Err(b)) => assert_eq!(
            core::mem::discriminant(&a),
            core::mem::discriminant(&b),
            "nondeterministic error at week={week}, secs={secs}, nanos={nanos}"
        ),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
            panic!("nondeterministic result at week={week}, secs={secs}, nanos={nanos}")
        }
    }
}

// ── Expected classification ──────────────────────────────────────────────────

/// The constructor's documented outcome for a given `(week, secs, nanos)`
/// triple, independent of the concrete `GnssTimeError` payload.
#[derive(Debug, Clone, Copy, PartialEq)]
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

        let week = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let secs = u64::from_le_bytes([
            data[5], data[6], data[7], data[8], data[9], data[10], data[11], data[12],
        ]);

        let nanos = u32::from_le_bytes([data[13], data[14], data[15], data[16]]);

        check_constructor(week, secs, nanos);

        return;
    }

    // BOUNDARY mode: structured exploration around every constructor edge.
    if data.len() < 16 {
        return;
    }

    let week = sat_add(
        u64::from(WEEK_EDGES[data[1] as usize % WEEK_EDGES.len()]),
        fold(
            i64::from(i32::from_le_bytes([data[4], data[5], data[6], data[7]])),
            8,
        ),
    );
    let secs = sat_add(
        SECS_EDGES[data[2] as usize % SECS_EDGES.len()],
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

    check_constructor(week as u32, secs, nanos as u32);
});
