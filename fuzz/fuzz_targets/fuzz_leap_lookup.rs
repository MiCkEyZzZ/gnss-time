#![no_main]

//! Fuzz target: `tai_minus_utc_at` lookup sanity for `LeapSeconds` and
//! `RuntimeLeapSeconds`.
//!
//! Property checks (no reference binary search): the returned TAI − UTC
//! offset must be **monotone non-decreasing** as the queried TAI instant
//! increases, and must always fall inside the **dynamic** offset range taken
//! from the table itself (`min..=max` of `entries()`, never a hardcoded
//! 19..=37). An empty runtime table is the documented fallback (always 19).
//!
//! Input layout (little-endian):
//!
//! - `data[0]` — mode flags:
//!   - `0x80` — runtime table starts from [`RuntimeLeapSeconds::from_builtin`]
//!     (19 entries); else empty.
//!   - `0x40` — build phase entries are RAW: 12 bytes each (`u64` `tai_nanos`
//!     + `i32` `tai_minus_utc`), appended via `try_extend` (errors ignored —
//!     building a query table is not the focus, `fuzz_try_extend` owns the
//!     contract).
//!   - `0x20` — as `0x40`, but BOUNDARY entries of 2 bytes each (`a` =
//!     `tai_nanos` delta as `i8` from the running last threshold, `b` = offset
//!     selector). Without `0x40`/`0x20` the table stays at its start state.
//!   - `0x10` — also run the same instant set through the static
//!     [`LeapSeconds::builtin`] provider.
//!   - `0x08` — lookup instants are BOUNDARY: 2 bytes each (`a` = threshold
//!     selector scaled to the built table, or `0xFF` for raw anchors; `b` =
//!     jitter as `i8` around the threshold). Otherwise RAW: 8 bytes each (`u64`
//!     TAI nanoseconds), full domain.
//! - `data[1]` — build-phase entry count (clamped to `RUNTIME_CAPACITY + 2`), a
//!   fixed split so the lookup instants can never be consumed by the build
//!   phase.
//! - `data[2..2 + count * step]` — build entries.
//! - remainder — lookup instants.
//!
//! All decoded lookup instants are collected, sorted by TAI and then every
//! adjacent pair is checked for monotonicity (a two-point scan never skips a
//! step since the table is strictly ascending with unit offsets).
//!
//! ## Run
//!
//! ```sh
//! cargo fuzz run fuzz_leap_lookup -- -max_total_time=300 -max_len=922 \
//!     -dict=fuzz_leap_lookup.dict
//! ```

use gnss_time::{
    LeapEntry, LeapSeconds, LeapSecondsProvider, RuntimeLeapSeconds, Tai, Time, RUNTIME_CAPACITY,
};
use libfuzzer_sys::fuzz_target;

/// Realistic number of lookup instants decoded and checked per input.
const MAX_INSTANTS: usize = 16;

/// Build-phase entries: enough to drive an empty table to full, and to grow a
/// builtin table well past `RUNTIME_CAPACITY`.
const MAX_BUILD_ENTRIES: usize = RUNTIME_CAPACITY + 2;

/// RAW entry byte size: `u64` `tai_nanos` + `i32` `tai_minus_utc`.
const RAW_ENTRY_BYTES: usize = 12;

/// TAI − UTC for an empty runtime table (documented fallback).
const EMPTY_FALLBACK_OFFSET: i32 = 19;

const BUILTIN_LAST_OFFSET: i32 = 37;

/// BOUNDARY build `tai_nanos`: last threshold ± `a` (as `i8`), clamped to
/// `u64`. `a == 0` keeps the previous threshold (rejected by `try_extend`),
/// `a > 0` walks strictly ascending, `a < 0` goes back down.
#[inline]
fn boundary_tai(
    last_tai: u64,
    a: u8,
) -> u64 {
    let dj = a as i8;

    if dj >= 0 {
        last_tai.saturating_add(dj as u64)
    } else {
        last_tai.saturating_sub((-i64::from(dj)) as u64)
    }
}

/// BOUNDARY build `tai_minus_utc`: `b & 0x07` selects a value relative to the
/// last offset or an absolute anchor (mirrors the `fuzz_try_extend` scheme).
#[inline]
fn boundary_offset(
    last_off: i32,
    b: u8,
) -> i32 {
    let sel = b & 0x07;

    let candidate: i64 = match sel {
        0 => i64::from(last_off) + 1, // valid +1 increment
        1 => i64::from(last_off),     // fails NonUnitIncrement
        2 => i64::from(last_off) + 2,
        3 => i64::from(i32::MAX),
        4 => i64::from(i32::MIN),
        5 => 0,
        6 => i64::from(BUILTIN_LAST_OFFSET),
        _ => i64::from(EMPTY_FALLBACK_OFFSET),
    };

    candidate.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// BOUNDARY lookup instant around the built table's thresholds or a raw
/// anchor (`a == 0xFF`).
#[inline]
fn boundary_instant(
    a: u8,
    b: u8,
    entries: &[LeapEntry],
) -> u64 {
    match a {
        0xFF => match b {
            0 => 0,
            1 => u64::MAX,
            2 => 1 << 63,
            3 => 1_467_264_000_000_000_000, // ~2026 "now"
            _ => u64::from(b),
        },
        _ => match entries.first() {
            None => u64::from(b),
            Some(_) => {
                let idx = (usize::from(a) * entries.len()) / 256;
                let base = entries[idx].tai_nanos;
                let dj = b as i8;

                if dj >= 0 {
                    base.saturating_add(dj as u64)
                } else {
                    base.saturating_sub((-i64::from(dj)) as u64)
                }
            }
        },
    }
}

/// Sorts by TAI and checks monotonicity + dynamic range for every instant.
fn check_lookup<P: LeapSecondsProvider>(
    provider: &P,
    entries: &[LeapEntry],
    instants: &[u64],
) {
    let n = instants.len().min(MAX_INSTANTS);
    let mut pairs = [(0u64, 0i32); MAX_INSTANTS];

    for (i, &tai) in instants.iter().take(n).enumerate() {
        pairs[i] = (tai, provider.tai_minus_utc_at(Time::<Tai>::from_nanos(tai)));
    }

    pairs[..n].sort_unstable_by_key(|p| p.0);

    for i in 1..n {
        assert!(
            pairs[i - 1].1 <= pairs[i].1,
            "tai_minus_utc_at non-monotonic: {} -> {} at TAI {} -> {}",
            pairs[i - 1].1,
            pairs[i].1,
            pairs[i - 1].0,
            pairs[i].0
        );
    }

    if entries.is_empty() {
        for &(_, off) in &pairs[..n] {
            assert_eq!(
                off, EMPTY_FALLBACK_OFFSET,
                "empty runtime table must always report the documented fallback"
            );
        }

        return;
    }

    let min = entries.iter().map(|e| e.tai_minus_utc).min().unwrap();
    let max = entries.iter().map(|e| e.tai_minus_utc).max().unwrap();

    for &(tai, off) in &pairs[..n] {
        assert!(
            min <= off && off <= max,
            "offset {off} at TAI {tai} outside dynamic table range [{min}, {max}]"
        );
    }
}

/// Verifies the runtime table is still internally consistent after building
/// (guards against corrupt state leaking through to lookups).
fn assert_table<'a>(rt: &'a RuntimeLeapSeconds) -> &'a [LeapEntry] {
    let entries = rt.entries();

    assert_eq!(rt.len(), entries.len(), "len()/entries() divergence");

    for pair in entries.windows(2) {
        assert!(
            pair[0].tai_nanos < pair[1].tai_nanos,
            "table not strictly ascending"
        );
        assert_eq!(
            i64::from(pair[1].tai_minus_utc),
            i64::from(pair[0].tai_minus_utc) + 1,
            "tai_minus_utc jumped: {} -> {}",
            pair[0].tai_minus_utc,
            pair[1].tai_minus_utc
        );
    }

    if let Some(last) = entries.last() {
        assert_eq!(rt.current_tai_minus_utc(), last.tai_minus_utc);
    }

    entries
}

fuzz_target!(|data: &[u8]| {
    let Some(&flags) = data.first() else {
        return; // libFuzzer feeds the empty input first
    };

    let start_builtin = flags & 0x80 != 0;
    let build_raw = flags & 0x40 != 0;
    let build_boundary = flags & 0x20 != 0;
    let check_static = flags & 0x10 != 0;
    let boundary_instants = flags & 0x08 != 0;
    // Fixed split: `data[1]` is the build count, everything after the build
    // region is lookup instants. Too-short inputs naturally produce fewer
    // entries/instants.
    let build_count = usize::from(data.get(1).copied().unwrap_or(0)).min(MAX_BUILD_ENTRIES);

    let mut rt = if start_builtin {
        RuntimeLeapSeconds::from_builtin()
    } else {
        RuntimeLeapSeconds::new()
    };

    let mut pos = 2usize;

    // Build phase: optionally grow the runtime table from the input.
    if build_raw || build_boundary {
        let step = if build_raw { RAW_ENTRY_BYTES } else { 2 };
        let avail = data.len().saturating_sub(pos) / step;
        let count = build_count.min(avail);

        for _ in 0..count {
            let base = pos;

            let entry = if build_raw {
                let tai = u64::from_le_bytes([
                    data[base],
                    data[base + 1],
                    data[base + 2],
                    data[base + 3],
                    data[base + 4],
                    data[base + 5],
                    data[base + 6],
                    data[base + 7],
                ]);
                let off = i32::from_le_bytes([
                    data[base + 8],
                    data[base + 9],
                    data[base + 10],
                    data[base + 11],
                ]);

                LeapEntry::new(tai, off)
            } else {
                let (last_tai, last_off) = match rt.entries().last() {
                    Some(e) => (e.tai_nanos, e.tai_minus_utc),
                    None => (0, 0),
                };
                LeapEntry::new(
                    boundary_tai(last_tai, data[base]),
                    boundary_offset(last_off, data[base + 1]),
                )
            };

            let _ = rt.try_extend(entry); // errors ignored (fuzz_try_extend owns the contract)

            pos += step;
        }
    }

    // The built table drives the BOUNDARY instant anchors.
    let entries = assert_table(&rt);
    // Lookup phase: decode instants, then property-check them.
    let step = if boundary_instants { 2 } else { 8 };
    let avail = data.len().saturating_sub(pos) / step;
    let count = avail.min(MAX_INSTANTS);
    let mut instants = [0u64; MAX_INSTANTS];

    for i in 0..count {
        let base = pos + i * step;

        instants[i] = if boundary_instants {
            boundary_instant(data[base], data[base + 1], entries)
        } else {
            u64::from_le_bytes([
                data[base],
                data[base + 1],
                data[base + 2],
                data[base + 3],
                data[base + 4],
                data[base + 5],
                data[base + 6],
                data[base + 7],
            ])
        };
    }

    check_lookup(&rt, entries, &instants[..count]);

    if check_static {
        let builtin = LeapSeconds::builtin();

        check_lookup(builtin, builtin.entries(), &instants[..count]);
    }
});
