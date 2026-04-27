# gnss-time

![Crates.io](https://img.shields.io/crates/v/gnss-time)
![no_std](https://img.shields.io/badge/no__std-yes-blue)
[![docs.rs](https://docs.rs/gnss-time/badge.svg)](https://docs.rs/gnss-time)

**Type-safe time handling for GNSS systems.**

`gnss-time` is a minimal, zero-cost core library for working with time in satellite
navigation systems such as GLONASS, GPS, Galileo, and BeiDou.

This is **not** a GNSS engine and **not** an RTK library.
It is a strictly typed time model designed to prevent domain-mixing bugs at compile
time.

## Features

- **Type-safe time scales**
  `Glonass`, `Gps`, `Galileo`, `Beidou`, `Tai`, `Utc`

- **Full 6×6 conversion matrix (30 directions)**
  All time scale conversions are explicitly defined and verified

- **Typed conversion API**
  - `IntoScale` — fixed-offset conversions
  - `IntoScaleWith` — leap-second-aware conversions

- **Explicit leap second handling**
  - No hidden global state
  - Detection of ambiguity during leap insertion
  - `ConvertResult<T>` for safe handling

- **Deterministic conversions via TAI pivot**

- **Domain-specific formats**
  - `Day:TOD` (GLONASS)
  - `Week:TOW` (GPS, Galileo, BeiDou)

- **Runtime conversion graph inspection**
  - `ConversionMatrix`
  - `ScaleId`, `ConversionKind`

- **High-precision durations** (`Duration`, nanoseconds)

- **Zero-cost abstractions**
  - timestamps are 8 bytes (`u64`)

- **`no_std` by default**
  suitable for embedded and real-time systems

## Example

```rust
use gnss_time::prelude::*;

let gps = Time::<Gps>::from_week_tow(2200, 0.0).unwrap();
let ls = LeapSeconds::builtin();

match gps.into_scale_with_checked(ls).unwrap() {
    ConvertResult::Exact(utc) => println!("UTC: {}", utc),
    ConvertResult::AmbiguousLeapSecond(utc) => {
        println!("Inside leap second, UTC value: {}", utc);
    }
}
```

## Design Goals

- Prevent mixing incompatible time domains at compile time
- Make leap seconds explicit and impossible to ignore
- Guarantee deterministic conversions
- Provide zero-cost abstractions over raw timestamps
- Be fully usable in `no_std` environments
- Serve as a foundational building block for GNSS software

## Supported Time Scales

| Scale   | Epoch              | Format        | Offset vs TAI         |
| ------- | ------------------ | ------------- | --------------------- |
| GLONASS | 1996-01-01 UTC(SU) | `GLO D:TOD`   | contextual (needs LS) |
| GPS     | 1980-01-06 UTC     | `GPS W:TOW`   | +19 s (fixed)         |
| Galileo | 1999-08-22 UTC     | `GAL W:TOW`   | +19 s (fixed)         |
| BeiDou  | 2006-01-01 UTC     | `BDT W:TOW`   | +33 s (fixed)         |
| TAI     | 1958-01-01         | `TAI +Ss Nns` | 0 s (pivot)           |
| UTC     | 1972-01-01         | `UTC +Ss Nns` | contextual (needs LS) |

## Why not use standard libraries?

Typical time libraries:

- do not distinguish GNSS time domains
- allow unsafe mixing of GPS / UTC / TAI
- hide leap seconds or ignore them entirely
- are not `no_std` compatible

`gnss-time` solves these problems at the type level.

## Status

- [x] Core types (`Time<S>`, `Duration`)
- [x] Epoch definitions
- [x] Fixed-offset conversions
- [x] Leap second handling
- [x] Display formats
- [x] Full conversion matrix

## License

Dual-licensed under:

- MIT
- Apache-2.0
