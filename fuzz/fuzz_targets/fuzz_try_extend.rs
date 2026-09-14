#![no_main]

//! Fuzz target: `RuntimeLeapSeconds::try_extend` contract.
//!
//! This is the only #TIME-18 API path that can panic / corrupt state, so it
//! gets its own target. The harness simulates the *same* validation that
//! `try_extend` performs using i64 arithmetic (immune to i32 wraparound) and
//! asserts that the implementation returns exactly the expected error, in the
//! documented priority order, on every input sequence.
//!
//! Two input modes combine orthogonally with the starting table:
//!
//! - **start state** (`data[0] & 0x80 != 0`): begin from
//!   `RuntimeLeapSeconds::from_builtin()` (19 entries, last `tai_minus_utc` =
//!   37); otherwise begin empty.
//! - **entry encoding** (`data[0] & 0x40 != 0`): BOUNDARY — two bytes per
//!   entry, `tai_nanos` offset relative to the current last threshold and a
//!   fixed offset selector; RAW — twelve bytes per entry (`u64` `tai_nanos` +
//!   `i32` `tai_minus_utc`), full domain.
//!
//! Invariants (mirroring the documented contract):
//! 1. No panic on any input sequence (beyond the *known* i32-wraparound defect
//!    in `try_extend`: `last.tai_minus_utc + 1` wraps or panics when the last
//!    offset is `i32::MAX`, which is reachable from an empty table in a single
//!    first entry — the harness models it with i64 and reports the mismatch /
//!    abort as a crash).
//! 2. `BufferFull` is returned **exactly** when `len == RUNTIME_CAPACITY` (65th
//!    `Ok` succeeds the 64th), never earlier or later.
//! 3. `NotStrictlyAscending` is returned iff `entry.tai_nanos <= last`.
//! 4. `NonUnitIncrement` is returned iff `entry.tai_minus_utc != last + 1`.
//! 5. On success the table stays strictly ascending with unit increments;
//!    `len()`, `entries()`, `current_tai_minus_utc()`, `last_update()` and the
//!    `tai_minus_utc_at` binary search stay consistent on and around the last
//!    threshold.
//!
//! ## Run
//!
//! ```sh
//! cargo fuzz run fuzz_try_extend -- -max_total_time=300 -max_len=793 \
//!     -dict=fuzz_try_extend.dict
//! ```

use gnss_time::{
    LeapEntry, LeapExtendError, LeapSecondsProvider, RuntimeLeapSeconds, Tai, Time,
    RUNTIME_CAPACITY,
};
use libfuzzer_sys::fuzz_target;

/// Entries processed per input: enough to drive an empty table to full and
/// observe `BufferFull` on the next call, from either starting state.
const MAX_ENTRIES: usize = RUNTIME_CAPACITY + 2;

/// Raw entry byte size: `u64` `tai_nanos` + `i32` `tai_minus_utc`.
const RAW_ENTRY_BYTES: usize = 12;

const BUILTIN_LAST_OFFSET: i32 = 37;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpectedKind {
    Ok,
    BufferFull,
    NotStrictlyAscending,
    NonUnitIncrement,
    OffsetOverflow,
}

/// Mirrors the implementation's validation with i64 arithmetic so i32
/// wraparound (the `i32::MAX + 1` defect) surfaces as a mismatch instead of
/// silently passing.
#[derive(Clone, Copy, Debug)]
struct Model {
    len: usize,
    last_tai: u64,
    last_off: i32,
}

impl Model {
    fn from_entries(entries: &[LeapEntry]) -> Self {
        let len = entries.len();

        if len == 0 {
            return Self {
                len,
                last_tai: 0,
                last_off: 0,
            };
        }

        let last = entries[len - 1];

        Self {
            len,
            last_tai: last.tai_nanos,
            last_off: last.tai_minus_utc,
        }
    }

    fn expect(
        &self,
        entry: LeapEntry,
    ) -> ExpectedKind {
        if self.len >= RUNTIME_CAPACITY {
            return ExpectedKind::BufferFull;
        }

        if self.len > 0 {
            if entry.tai_nanos <= self.last_tai {
                return ExpectedKind::NotStrictlyAscending;
            }

            // i64 `last + 1` never overflows, but the implementation must
            // report OffsetOverflow when the real last offset is i32::MAX:
            // no valid successor exists to satisfy the +1 unit increment.
            let need = i64::from(self.last_off) + 1;
            if need > i64::from(i32::MAX) {
                return ExpectedKind::OffsetOverflow;
            }

            if i64::from(entry.tai_minus_utc) != need {
                return ExpectedKind::NonUnitIncrement;
            }
        }

        ExpectedKind::Ok
    }

    fn apply(
        &mut self,
        entry: LeapEntry,
    ) {
        debug_assert!(self.len < RUNTIME_CAPACITY);

        self.last_tai = entry.tai_nanos;
        self.last_off = entry.tai_minus_utc;
        self.len += 1;
    }
}

fn actual_kind(result: Result<(), LeapExtendError>) -> ExpectedKind {
    match result {
        Ok(()) => ExpectedKind::Ok,
        Err(LeapExtendError::BufferFull) => ExpectedKind::BufferFull,
        Err(LeapExtendError::NotStrictlyAscending) => ExpectedKind::NotStrictlyAscending,
        Err(LeapExtendError::NonUnitIncrement) => ExpectedKind::NonUnitIncrement,
        Err(LeapExtendError::OffsetOverflow) => ExpectedKind::OffsetOverflow,
        Err(LeapExtendError::EmptyTable) => {
            panic!("try_extend returned EmptyTable (not part of its contract)")
        }
        Err(_) => panic!("try_extend returned an unknown LeapExtendError variant"),
    }
}

/// BOUNDARY `tai_nanos`: last threshold ± `a` (as i8), clamped to `u64`.
/// Sign-equal to <= / > walking: `a == 0` → not strictly ascending,
/// `a > 0` → strictly ascending, `a < 0` → not strictly ascending.
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

/// BOUNDARY `tai_minus_utc`: `b & 0x07` selects a value relative to the last
/// offset or an absolute anchor. Clamped so `last == i32::MAX` never overflows
/// the selector itself.
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
        3 => i64::from(i32::MAX), // overflow trigger seed
        4 => i64::from(i32::MIN),
        5 => 0,
        6 => i64::from(BUILTIN_LAST_OFFSET),
        _ => 19,
    };
    candidate.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Post-success + cross-iteration state checks against the model.
fn assert_invariants(
    rt: &RuntimeLeapSeconds,
    model: &Model,
) {
    assert_eq!(rt.len(), model.len, "len diverged from model");
    let entries = rt.entries();
    assert_eq!(entries.len(), model.len, "entries().len() diverged");

    // Strictly ascending thresholds, unit `tai_minus_utc` increments (i64 math
    // so the wraparound defect cannot hide behind i32 overflow).
    for pair in entries.windows(2) {
        assert!(
            pair[0].tai_nanos < pair[1].tai_nanos,
            "table not strictly ascending between {} and {}",
            pair[0].tai_nanos,
            pair[1].tai_nanos
        );
        assert_eq!(
            i64::from(pair[1].tai_minus_utc),
            i64::from(pair[0].tai_minus_utc) + 1,
            "tai_minus_utc jumped: {} -> {}",
            pair[0].tai_minus_utc,
            pair[1].tai_minus_utc
        );
    }

    if model.len > 0 {
        let last = &entries[model.len - 1];
        assert_eq!(last.tai_nanos, model.last_tai, "last threshold diverged");
        assert_eq!(last.tai_minus_utc, model.last_off, "last offset diverged");
        assert_eq!(
            rt.current_tai_minus_utc(),
            model.last_off,
            "current_tai_minus_utc diverged"
        );
    }

    // Binary search consistency around the last threshold.
    if model.len > 1 {
        let last_tai = model.last_tai;
        let prev_off = entries[model.len - 2].tai_minus_utc;
        if last_tai > 0 {
            assert_eq!(
                rt.tai_minus_utc_at(Time::<Tai>::from_nanos(last_tai - 1)),
                prev_off,
                "lookup just below last threshold wrong"
            );
        }
        if last_tai < u64::MAX {
            assert_eq!(
                rt.tai_minus_utc_at(Time::<Tai>::from_nanos(last_tai + 1)),
                model.last_off,
                "lookup just above last threshold wrong"
            );
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Some(&flags) = data.first() else {
        return; // libFuzzer feeds the empty input first
    };

    let start_builtin = flags & 0x80 != 0;
    let boundary = flags & 0x40 != 0;

    let mut rt = if start_builtin {
        RuntimeLeapSeconds::from_builtin()
    } else {
        RuntimeLeapSeconds::new()
    };
    let mut model = Model::from_entries(rt.entries());

    let step = if boundary { 2 } else { RAW_ENTRY_BYTES };
    let count = usize::min((data.len() - 1) / step, MAX_ENTRIES);

    for i in 0..count {
        let base = 1 + i * step;

        let entry = if boundary {
            let tai = boundary_tai(model.last_tai, data[base]);
            let off = boundary_offset(model.last_off, data[base + 1]);
            LeapEntry::new(tai, off)
        } else {
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
        };

        let expected = model.expect(entry);
        let actual = actual_kind(rt.try_extend(entry));

        // The model and implementation share "same checks, different width".
        assert_eq!(
            actual, expected,
            "try_extend classification mismatch (i={i}, entry=({}, {}), \
             start_builtin={start_builtin}, boundary={boundary}): \
             expected={expected:?}, got={actual:?}, len={}",
            entry.tai_nanos, entry.tai_minus_utc, model.len
        );

        if actual == ExpectedKind::Ok {
            model.apply(entry);
        }

        assert_invariants(&rt, &model);
    }
});
