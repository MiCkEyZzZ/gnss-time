use crate::LeapEntry;

// Source: IERS Bulletin C
// https://hpiers.obspm.fr/iers/bul/bulc/Leap_Second.dat
//
// Last verified: IERS Bulletin C 70 (December 2024)
//   — no leap second scheduled through June 2025.
// Current status (May 2026): TAI − UTC = 37 s, unchanged since 2017-01-01.
//
// Each entry: (threshold_tai_nanos, TAI-UTC after this moment)
//
// Formula:
//   threshold_tai_nanos = (unix_event - GPS_EPOCH_UNIX + tai_minus_utc) × 10⁹
//   where GPS_EPOCH_UNIX = 315_964_800
//
// Cross-reference (unix → threshold):
//   1981-07-01  362_793_600  → ( 46_828_800 + 20) × 1e9 =
// 46_828_820_000_000_000   1982-07-01  394_329_600  → ( 78_364_800 + 21) × 1e9
// =  78_364_821_000_000_000   1983-07-01  425_865_600  → (109_900_800 + 22) ×
// 1e9 = 109_900_822_000_000_000   1985-07-01  489_024_000  → (173_059_200 + 23)
// × 1e9 = 173_059_223_000_000_000   1988-01-01  567_993_600  → (252_028_800 +
// 24) × 1e9 = 252_028_824_000_000_000   1990-01-01  631_152_000  → (315_187_200
// + 25) × 1e9 = 315_187_225_000_000_000   1991-01-01  662_688_000  →
// (346_723_200 + 26) × 1e9 = 346_723_226_000_000_000   1992-07-01  709_948_800
// → (393_984_000 + 27) × 1e9 = 393_984_027_000_000_000   1993-07-01
// 741_484_800  → (425_520_000 + 28) × 1e9 = 425_520_028_000_000_000
//   1994-07-01  773_020_800  → (457_056_000 + 29) × 1e9 =
// 457_056_029_000_000_000   1996-01-01  820_454_400  → (504_489_600 + 30) × 1e9
// = 504_489_630_000_000_000   1997-07-01  867_715_200  → (551_750_400 + 31) ×
// 1e9 = 551_750_431_000_000_000   1999-01-01  915_148_800  → (599_184_000 + 32)
// × 1e9 = 599_184_032_000_000_000   2006-01-01 1_136_073_600 → (820_108_800 +
// 33) × 1e9 = 820_108_833_000_000_000   2009-01-01 1_230_768_000 → (914_803_200
// + 34) × 1e9 = 914_803_234_000_000_000   2012-07-01 1_341_100_800 →
// (1_025_136_000+35) × 1e9 =1_025_136_035_000_000_000   2015-07-01
// 1_435_708_800 → (1_119_744_000+36) × 1e9 =1_119_744_036_000_000_000
//   2017-01-01 1_483_228_800 → (1_167_264_000+37) × 1e9
// =1_167_264_037_000_000_000

/// Built-in leap-second table, sourced from IERS Bulletin C.
///
/// Covers all 19 entries from the GPS epoch (1980-01-06, TAI−UTC = 19)
/// through 2017-01-01 (TAI−UTC = 37).
///
/// # Update policy
///
/// When IERS announces a new leap second via Bulletin C:
/// 1. Add a new `LeapEntry::new(threshold, n)` at the end of this array.
/// 2. Update the "Last verified" comment above.
/// 3. Run `cargo test` — the compile-time assertions below will catch any
///    ordering or monotonicity violation.
pub const BUILTIN_TABLE: [LeapEntry; 19] = [
    // Base value at GPS epoch: TAI−UTC = 19
    LeapEntry::new(0, 19),
    // 1981-07-01: TAI−UTC → 20
    LeapEntry::new(46_828_820_000_000_000, 20),
    // 1982-07-01: TAI−UTC → 21
    LeapEntry::new(78_364_821_000_000_000, 21),
    // 1983-07-01: TAI−UTC → 22
    LeapEntry::new(109_900_822_000_000_000, 22),
    // 1985-07-01: TAI−UTC → 23
    LeapEntry::new(173_059_223_000_000_000, 23),
    // 1988-01-01: TAI−UTC → 24
    LeapEntry::new(252_028_824_000_000_000, 24),
    // 1990-01-01: TAI−UTC → 25
    LeapEntry::new(315_187_225_000_000_000, 25),
    // 1991-01-01: TAI−UTC → 26
    LeapEntry::new(346_723_226_000_000_000, 26),
    // 1992-07-01: TAI−UTC → 27
    LeapEntry::new(393_984_027_000_000_000, 27),
    // 1993-07-01: TAI−UTC → 28
    LeapEntry::new(425_520_028_000_000_000, 28),
    // 1994-07-01: TAI−UTC → 29
    LeapEntry::new(457_056_029_000_000_000, 29),
    // 1996-01-01: TAI−UTC → 30
    LeapEntry::new(504_489_630_000_000_000, 30),
    // 1997-07-01: TAI−UTC → 31
    LeapEntry::new(551_750_431_000_000_000, 31),
    // 1999-01-01: TAI−UTC → 32
    LeapEntry::new(599_184_032_000_000_000, 32),
    // 2006-01-01: TAI−UTC → 33
    LeapEntry::new(820_108_833_000_000_000, 33),
    // 2009-01-01: TAI−UTC → 34
    LeapEntry::new(914_803_234_000_000_000, 34),
    // 2012-07-01: TAI−UTC → 35
    LeapEntry::new(1_025_136_035_000_000_000, 35),
    // 2015-07-01: TAI−UTC → 36
    LeapEntry::new(1_119_744_036_000_000_000, 36),
    // 2017-01-01: TAI−UTC → 37 (latest known; valid through at least 2026)
    LeapEntry::new(1_167_264_037_000_000_000, 37),
];

// Compile-time integrity assertions
//
// These fira during `cargo build` (not just `cargo test`), so a mis-ordered or
// duplicate entry is caught imediately rather than at runtime.

/// Verifies that the table is strictly sorted by `tai_nanos` (ascending) and
/// that every `tai_minus_utc` value increments by exactly 1.
///
/// Panics at compile time if either invariant is violated.
#[allow(dead_code)]
const fn assert_table_invariants(table: &[LeapEntry]) {
    // Need at least one entry.
    assert!(!table.is_empty(), "BUILTIN_TABLE must not be empty");

    let mut i = 1;

    while i < table.len() {
        // Strict ascending order of thresholds.
        assert!(
            table[i].tai_nanos > table[i - 1].tai_nanos,
            "BUILTIN_TABLE: tai_nanos must be strictly ascending",
        );
        // Each entry adds exactly one leap second.
        assert!(
            table[i].tai_minus_utc == table[i - 1].tai_minus_utc + 1,
            "BUILTIN_TABLE: tai_minus_utc must increment by exactly 1",
        );
        i += 1;
    }
}

/// Compile-time assertion: table starts at GPS epoch with TAI−UTC = 19.
const _ASSERT_FIRST_ENTRY: () = {
    assert!(
        BUILTIN_TABLE[0].tai_nanos == 0,
        "BUILTIN_TABLE: first entry must have tai_nanos == 0"
    );
    assert!(
        BUILTIN_TABLE[0].tai_minus_utc == 19,
        "BUILTIN_TABLE: first entry must have tai_minus_utc == 19"
    );
};

/// Compile-time assertion: full table invariants (sorted, monotone).
const _ASSERT_TABLE_INVARIANTS: () = assert_table_invariants(&BUILTIN_TABLE);

/// Compile-time assertion: last known entry is 2017-01-01, TAI−UTC = 37.
const _ASSERT_LAST_ENTRY: () = {
    let last = BUILTIN_TABLE[BUILTIN_TABLE.len() - 1];
    assert!(
        last.tai_nanos == 1_167_264_037_000_000_000,
        "BUILTIN_TABLE: last entry threshold mismatch"
    );
    assert!(
        last.tai_minus_utc == 37,
        "BUILTIN_TABLE: last entry must have tai_minus_utc == 37"
    );
};
