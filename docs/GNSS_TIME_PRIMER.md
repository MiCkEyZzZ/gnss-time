# GNSS Time Primer

A practical guide to GNSS time systems for software developers.

## Why time in GNSS is hard

In most programs time is a single concept. You call `now()`, get a number, and
that number means the same thing everywhere.

In GNSS that is not the case.

Every satellite navigation system maintains its own independent clock. These
clocks start on different calendar dates, are anchored to different standards,
and accumulate different integer offsets over time. The same physical moment
in the real world has a different numeric value depending on which system you
ask.

This is not a quirk you can "paper over". An error means your receiver's
timestamps differ by 14, 18, or 19 seconds — and you would not even notice.

## Four time scales you need to understand

### TAI — International Atomic Time

TAI (Temps Atomique International) is the basis of all modern time.

- Maintained by the weighted average of ~450 atomic clocks worldwide
- Contains no leap seconds. An absolutely uniform scale
- Epoch: **1958-01-01 00:00:00**
- All GNSS systems are tied to TAI by a fixed or computed offset

TAI is the **reference scale** for all conversions in this library. Any
conversion between GNSS scales passes through TAI internally.

### UTC — Coordinated Universal Time

UTC is what your wall clock shows (approximately).

- UTC is kept close to UT1 (Earth rotation)
- When Earth's rotation slows, a **leap second** is added
- UTC = TAI − N, where N is the number of inserted leap seconds
- As of 2017: **TAI − UTC = 37 seconds**

Leap seconds are announced by the IERS in advance (about 6 months before).
There is no formula to predict them — a table is required.

### GLONASS time

GLONASS is the Russian navigation system.

- Epoch: **1996-01-01 00:00:00 UTC(SU)** = 1995-12-31 21:00:00 UTC
- GLONASS tracks **UTC(SU)** — the Russian realization of UTC, which equals
  UTC + 3 hours (Moscow time), **but includes leap seconds**
- Because GLONASS is synchronized to UTC, the GLONASS <-> UTC conversion is
  just an epoch shift: **+757 371 600 seconds**
- The leap-second table is not needed for GLONASS <-> UTC
- But it **is** needed for GLONASS <-> GPS / Galileo / BeiDou

### GPS time

GPS is the American navigation system.

- Epoch: **1980-01-06 00:00:00 UTC**
- Offset: **GPS = TAI − 19 seconds** (fixed, no leap seconds)
- GPS was synchronized to UTC at its epoch, when TAI − UTC = 19 s
- No leap seconds have ever been inserted into GPS — time runs continuously
- As of 2017: **GPS is 18 seconds ahead of UTC**

GPS receivers transmit the current GPS–UTC offset in navigation messages so
that civil time can be computed.

### Galileo time

Galileo is the European navigation system.

- Epoch: **1999-08-22 00:00:00 UTC** (GPS week 1024, TOW 0)
- Offset: **Galileo = TAI − 19 seconds** — identical to GPS
- A Galileo and GPS nanosecond with the same value **represent the same
  physical moment**
- The GPS <-> Galileo conversion is identity: the numeric nanosecond value
  does not change

### BeiDou time

BDT is the Chinese navigation system.

- Epoch: **2006-01-01 00:00:00 UTC**
- Offset: **BDT = TAI − 33 seconds**
- Since GPS = TAI − 19 s, we get: **BDT = GPS − 14 seconds**
- This 14-second difference is fixed and will never change

## The leap-second problem

Leap seconds are the hardest part of working with GNSS time.

### What is a leap second?

When Earth's rotation slows, UTC begins to run ahead of UT1 (solar time). To
keep the difference within 0.9 seconds, the IERS sometimes inserts a
**positive leap second**:

UTC clocks show **23:59:60** before rolling over to **00:00:00**.

Negative leap seconds (removing a second) are theoretically possible, but have
never been applied.

### How GPS handles leap seconds

GPS has no leap seconds. Time just increases.

When a leap second is inserted into UTC, the GPS–UTC difference increases by 1.

Before 1981-07-01 the difference was 0. After 2017-01-01 it is 18 seconds.

| Event      | TAI − UTC | GPS − UTC |
| ---------- | --------- | --------- |
| 1980-01-06 | 19 s      | 0 s       |
| 1981-07-01 | 20 s      | 1 s       |
| ...        | ...       | ...       |
| 1999-01-01 | 32 s      | 13 s      |
| 2017-01-01 | 37 s      | 18 s      |

GPS receivers transmit the current GPS – UTC offset (the `IODC` field) so that
software can compute civil time.

### The 1-second ambiguity window

At the moment of a leap-second insertion there is a 1-second window in which
the same GPS timestamp corresponds to two UTC values:

```zsh
GPS: 1_167_264_017 ns  →  UTC: 23:59:59  (last second before the leap second)
GPS: 1_167_264_018 ns  →  UTC: 23:59:60  (the inserted leap second)
GPS: 1_167_264_018 ns  →  UTC: 00:00:00  (start of the next day — same GPS value!)
```

This library detects the window and signals it via
`ConvertResult::AmbiguousLeapSecond`.

## Conversion graph

```text
                    ┌──────────────────────────────────────────────┐
                    │           TAI (reference scale)              │
                    │  T_tai = T_self + OFFSET_TO_TAI              │
                    └──────┬──────┬───────┬──────┬─────────────────┘
                           │      │       │      │
               fixed +19s  │      │+19s   │+33s  │  contextual
                           ▼      ▼       ▼      │
                          GPS   Galileo  BeiDou  │
                           │      │       │      │
                           │ identity fixed     │
                           │                     ▼
                           │               UTC ←──── GLONASS
                           │               │  epoch shift
                           └───────────────┘
                            contextual (needs the leap-second table)
```

Fixed conversions (no leap seconds required):

- GPS ↔ TAI, GPS ↔ Galileo, GPS ↔ BeiDou
- Galileo ↔ BeiDou, Galileo ↔ TAI, BeiDou ↔ TAI
- GLONASS ↔ UTC

Contextual conversions (a `LeapSecondsProvider` is required):

- GPS ↔ UTC, GPS ↔ GLONASS
- Galileo ↔ UTC, Galileo ↔ GLONASS
- BeiDou ↔ UTC, BeiDou ↔ GLONASS

## Common mistakes

### Using GPS as UTC

```rust
// WRONG — GPS is 18 seconds ahead of UTC after 2017
let gps = Time::<Gps>::from_seconds(gps_seconds_from_receiver);
let civil_time = gps.as_seconds(); // ← these are GPS seconds, not UTC!
```

```rust
// RIGHT
let utc = gps.into_scale_with(LeapSeconds::builtin()).unwrap();
let civil_seconds = utc.as_seconds(); // UTC seconds since 1972-01-01
```

### Ignoring leap seconds in GPS → UTC

```rust
// WRONG — assumes a fixed 18 seconds
let utc_seconds = gps.as_seconds() - 18;
```

```rust
// RIGHT — uses the full leap-second table
let utc = gps.into_scale_with(LeapSeconds::builtin()).unwrap();
```

### Mixing time scales in arithmetic

```rust
// WRONG — will not compile in gnss-time, but a common mistake:
// let delta = gps_time - glonass_time;
```

```rust
// RIGHT — first convert to a single scale
let glo_as_utc: Time<Utc> = glonass.into_scale().unwrap();
let gps_as_utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
let delta = gps_as_utc - glo_as_utc;
```

## Sources

- IS-GPS-200 Rev. N (2022) — GPS interface specification
- ICD-GLONASS v5.1 (2008) — GLONASS interface control document
- OS-SIS-ICD Issue 2.0 (2021) — Galileo ICD
- BDS-SIS-ICD-B1I-3.0 (2019) — BeiDou ICD
- IERS Bulletin C — leap-second announcements: [https://www.iers.org](https://www.iers.org)
- Howard Hinnant, "Date Algorithms": [http://howardhinnant.github.io/date_algorithms.html](http://howardhinnant.github.io/date_algorithms.html)
