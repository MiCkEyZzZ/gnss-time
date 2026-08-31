# Leap Seconds

Documentation for the built-in leap-second table in `gnss-time`, the update
policy, and runtime extension.

## Data source

All data comes from IERS Bulletin C:
<https://hpiers.obspm.fr/iers/bul/bulc/Leap_Second.dat>

**Last verification:** IERS Bulletin C 70 (December 2024) — no new leap
seconds scheduled through June 2025.
**Current status (May 2026):** TAI − UTC = 37 s, unchanged since 2017-01-01.

## What a leap second is

TAI (International Atomic Time) is a continuous atomic time scale.
UTC is civil time, which is kept within 0.9 s of UT1 (astronomical time) by
periodically inserting **leap seconds**.

On insertion: UTC "stops" for one second — the instant
`23:59:60 UTC` exists, while GPS continues to run continuously. The
`TAI − UTC` difference increases by 1.

GPS never inserts or skips seconds: `GPS = TAI − 19 s` (fixed
since 1980-01-06). Therefore, a GPS ↔ UTC conversion requires knowing the
current `TAI − UTC` difference.

## Table format

Each entry is `(tai_nanos_threshold, tai_minus_utc)`:

- `tai_nanos_threshold` — the TAI nanoseconds (GPS-relative) from which the
  new value takes effect (inclusive, lower bound)
- `tai_minus_utc` — the `TAI − UTC` value in whole seconds from this threshold

### Threshold computation formula

```text
tai_nanos = (unix_event_timestamp − GPS_EPOCH_UNIX + tai_minus_utc) × 10⁹
```

where `GPS_EPOCH_UNIX = 315 964 800` (seconds of Unix time).

Example for 2017-01-01 (`unix = 1 483 228 800`, `n = 37`):

```text
gps_s     = 1_483_228_800 − 315_964_800 = 1_167_264_000
threshold = (1_167_264_000 + 37) × 10⁹  = 1_167_264_037_000_000_000
```

## Full table (19 entries, GPS era)

| #   | Event date | TAI−UTC | GPS−UTC | tai_nanos threshold      |
| --- | ---------- | ------- | ------- | ------------------------ |
| 0   | 1980-01-06 | 19      | 0       | 0                        |
| 1   | 1981-07-01 | 20      | 1       | 46 828 820 000 000 000   |
| 2   | 1982-07-01 | 21      | 2       | 78 364 821 000 000 000   |
| 3   | 1983-07-01 | 22      | 3       | 109 900 822 000 000 000  |
| 4   | 1985-07-01 | 23      | 4       | 173 059 223 000 000 000  |
| 5   | 1988-01-01 | 24      | 5       | 252 028 824 000 000 000  |
| 6   | 1990-01-01 | 25      | 6       | 315 187 225 000 000 000  |
| 7   | 1991-01-01 | 26      | 7       | 346 723 226 000 000 000  |
| 8   | 1992-07-01 | 27      | 8       | 393 984 027 000 000 000  |
| 9   | 1993-07-01 | 28      | 9       | 425 520 028 000 000 000  |
| 10  | 1994-07-01 | 29      | 10      | 457 056 029 000 000 000  |
| 11  | 1996-01-01 | 30      | 11      | 504 489 630 000 000 000  |
| 12  | 1997-07-01 | 31      | 12      | 551 750 431 000 000 000  |
| 13  | 1999-01-01 | 32      | 13      | 599 184 032 000 000 000  |
| 14  | 2006-01-01 | 33      | 14      | 820 108 833 000 000 000  |
| 15  | 2009-01-01 | 34      | 15      | 914 803 234 000 000 000  |
| 16  | 2012-07-01 | 35      | 16      | 1 025 136 035 000 000 000|
| 17  | 2015-07-01 | 36      | 17      | 1 119 744 036 000 000 000|
| 18  | 2017-01-01 | 37      | 18      | 1 167 264 037 000 000 000|

## Table update policy

IERS publishes Bulletin C twice a year (January and July). Each issue reports:

- whether a new leap second will be added in the next 6 months, or
- confirms that there are no changes.

### When to update

If IERS announces a new leap second:

1. **Compute the threshold** using the formula above, using the Unix timestamp
   of the event.
2. **Add an entry** to the end of the `BUILTIN_TABLE` array in
   `src/tables/leap_seconds.rs`:

   ```rust
   // YYYY-MM-DD: TAI−UTC → N
   LeapEntry::new(<threshold>, <N>),
   ```

3. **Update the `// Last verified:` comment** in the same file.
4. **Update the header** of this document (`LEAP_SECONDS.md`).
5. **Run the tests** — the compile-time assertions in `leap_seconds.rs`
   automatically verify ordering and monotonicity:

   ```bash
   cargo test
   cargo check --target thumbv7em-none-eabihf
   ```

6. **Update `CHANGELOG.md`** and release a patch version of the crate.

### Automatic table correctness checks

In `src/tables/leap_seconds.rs` three `const`-assertions are defined that fire
during **compilation** (not only at test time):

| Assertion                | What it checks                                      |
| ------------------------ | --------------------------------------------------- |
| `_ASSERT_FIRST_ENTRY`    | `tai_nanos == 0`, `tai_minus_utc == 19`             |
| `_ASSERT_TABLE_INVARIANTS`| strict ordering and a +1 increment across the whole table |
| `_ASSERT_LAST_ENTRY`     | the last entry matches 2017-01-01, `n == 37`        |

If you add an entry with an incorrect threshold or skip an increment, the
compiler rejects it **immediately**, without running tests.

> **Important:** when adding a new entry you must update `_ASSERT_LAST_ENTRY`
> — change the expected `tai_nanos` and `tai_minus_utc` values.

## Using the built-in table

```rust
use gnss_time::{LeapSeconds, gps_to_utc, LeapSecondsProvider};
use gnss_time::{Time, Gps, Tai};

// Built-in table: covers all GPS-era events up to 2017-01-01
let ls = LeapSeconds::builtin();

// Diagnostics: when was the last event?
let last = ls.last_update().unwrap();

assert_eq!(last.as_nanos(), 1_167_264_037_000_000_000); // 2017-01-01

// Current TAI−UTC value
assert_eq!(ls.current_tai_minus_utc(), 37);

// GPS → UTC conversion
let gps = Time::<Gps>::from_seconds(1_167_264_018); // 2017-01-01 GPS
let utc = gps_to_utc(gps, &ls).unwrap();
```

## Runtime update for embedded / receiver

For GNSS receivers that load the table from a navigation message or almanac,
use `RuntimeLeapSeconds`:

```rust
use gnss_time::{LeapEntry, RuntimeLeapSeconds, LeapSecondsProvider};

// Start from the compile-time snapshot of the built-in table
let mut rt = RuntimeLeapSeconds::from_builtin();

// The receiver reports a new second from the almanac
// (hypothetical event — illustrative only)
// rt.try_extend(LeapEntry::new(threshold_ns, 38)).unwrap();

// Used in the same places as LeapSeconds::builtin()
let gps = gnss_time::Time::<gnss_time::Gps>::from_seconds(1_000_000);
let utc = gnss_time::gps_to_utc(gps, &rt).unwrap();
```

### `RuntimeLeapSeconds` API

| Method                     | Description                            |
| -------------------------- | -------------------------------------- |
| `from_builtin()`           | Creates a table from the compile-time snapshot |
| `from_slice(&[LeapEntry])`| Creates from an arbitrary slice        |
| `try_extend(entry)`        | Adds a new entry with validation       |
| `last_update()`            | TAI moment of the last event           |
| `current_tai_minus_utc()`  | Current TAI−UTC value                  |
| `len()` / `is_empty()`     | Table size                             |
| `entries()`                | All entries as a slice                 |

### Validation on `try_extend`

`try_extend` rejects invalid entries:

```rust
use gnss_time::{LeapEntry, LeapExtendError, RuntimeLeapSeconds};

let mut rt = RuntimeLeapSeconds::from_builtin();

// Error: the threshold does not strictly increase
let err = rt.try_extend(LeapEntry::new(0, 38)).unwrap_err();

assert_eq!(err, LeapExtendError::NotStrictlyAscending);

// Error: the increment is not 1
let err = rt.try_extend(LeapEntry::new(9_999_999_999_000_000_000, 99)).unwrap_err();

assert_eq!(err, LeapExtendError::NonUnitIncrement);
```

## Custom provider

For full control, implement the `LeapSecondsProvider` trait:

```rust
use gnss_time::{LeapSecondsProvider, Tai, Time};

/// Provider with a fixed value (for example, for tests).
struct FixedOffset(i32);

impl LeapSecondsProvider for FixedOffset {
    fn tai_minus_utc_at(&self, _tai: Time<Tai>) -> i32 {
        self.0
    }
}

let provider = FixedOffset(37);
```

## Future leap seconds and discontinuation

The table has no entries after 2017-01-01. For later moments the library
returns `TAI − UTC = 37` (the last known value) — the standard
"assume no new leap seconds" approach.

**Discontinuation status:** At the 2022 World Radiocommunication Conference
(WRC-22) a decision was made to discontinue the insertion of leap seconds
**no later than 2035**. Until that deadline the current table remains valid.
After 2035 the UTC ↔ GPS conversion will become fully deterministic (like
GPS ↔ TAI today).

### IERS monitoring

For tracking new announcements:

- **Bulletin C:** <https://hpiers.obspm.fr/iers/bul/bulc/bulletinC.dat>
- **Latest data file:** <https://hpiers.obspm.fr/iers/bul/bulc/Leap_Second.dat>
- **IETF LEAPSECOND:** <https://www.ietf.org/timezones/data/leap-seconds.list>
