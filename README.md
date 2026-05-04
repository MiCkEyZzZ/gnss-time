# gnss-time

![Crates.io](https://img.shields.io/crates/v/gnss-time)
![no_std](https://img.shields.io/badge/no__std-yes-blue)
[![docs.rs](https://docs.rs/gnss-time/badge.svg)](https://docs.rs/gnss-time)
![MSRV](https://img.shields.io/badge/MSRV-1.75-blue)
![Embedded](https://img.shields.io/badge/embedded-friendly-green)

**Strongly typed GNSS time model with zero-cost conversions and explicit leap
second handling.**

`gnss-time` is a minimal, high-performance library for representing and converting
time across GNSS and atomic time scales:

- GLONASS
- GPS
- Galileo
- BeiDou
- TAI
- UTC

This crate focuses on **correctness, type safety, and deterministic conversions**,
not on navigation or positioning algorithms.

## Core Idea (mental model)

GNSS time is not a single system.

Each scale differs in:

- epoch
- unit definition
- leap second behavior

This crate enforces:

> different time scales are different types

So invalid mixing is impossible at compile time.

## API in 2 minutes

```rust
use gnss_time::prelude::*;

let gps = Time::<Gps>::from_week_tow(2200, DurationParts {
    seconds: 0,
    nanos: 0,
}).unwrap();

// Fixed conversion (zero-cost)
let gal: Time<Galileo> = gps.into_scale().unwrap();
```

## GNSS Time Primer

GNSS systems define different time scales:

- **GLONASS** → UTC(SU)-aligned (leap-second dependent)
- **GPS / Galileo** → TAI − 19s (fixed offset)
- **BeiDou (BDT)** → TAI − 33s (fixed offset)
- **TAI** → continuous atomic time
- **UTC** → civil time with leap seconds

> The same physical moment may have multiple valid representations.

## Features

### Type-safe time domains

Each scale is a distinct type:

- `Glonass`
- `Gps`
- `Galileo`
- `Beidou`
- `Tai`
- `Utc`

Cross-scale arithmetic is **not allowed implicitly**.

### Zero-cost abstractions

Arithmetic compiles down to native operations:

- `Time + Duration` ≈ `u64 + u64`
- no heap allocations
- no runtime overhead in fast path

### Explicit conversion model

Conversions are categorized:

- **Fixed** → constant offset (zero-cost)
- **EpochShift** → deterministic shift
- **Contextual** → leap second aware

### Leap-second aware UTC

UTC conversions support:

- leap second table lookup
- ambiguity detection
- explicit result model:

```rust
ConvertResult::Exact
ConvertResult::AmbiguousLeapSecond
```

### Conversion graph inspection

The library exposes the full conversion matrix:

- 6×6 scale graph
- fixed vs contextual edges
- runtime inspection tools

## Performance

### Arithmetic

| Operation                      | Cost    |
| ------------------------------ | ------- |
| `Time + Duration` (panic path) | ~0.5 ns |
| `checked_add`                  | ~4.3 ns |
| `saturating_add`               | ~0.5 ns |

### Conversions

| Operation                     | Cost        |
| ----------------------------- | ----------- |
| GPS → Galileo / TAI / BeiDou  | ~0.8–1.0 ns |
| GPS → UTC (leap-second aware) | ~9–10 ns    |
| UTC → GPS                     | ~22 ns      |

## Important design choice

### UTC is contextual

UTC conversions:

- depend on leap second table
- are not always invertible
- may be ambiguous during leap insertion

This is intentional and modeled explicitly.

## Example: leap-second aware conversion

```rust
use gnss_time::prelude::*;

let gps = Time::<Gps>::from_week_tow(
        2200,
        DurationParts {
            seconds: 0,
            nanos: 0,
        },
    )
    .unwrap();

let ls = LeapSeconds::builtin();
let result: ConvertResult<Time<Utc>> = gps.into_scale_with_checked(ls).unwrap();

match result {
    ConvertResult::Exact(utc) => {
        println!("UTC: {}", utc);
    }
    ConvertResult::AmbiguousLeapSecond(utc) => {
        println!("Leap second ambiguity: {}", utc);
    }
}
```

## No domain mixing guarantee

Invalid operations are rejected at compile time:

```rust
let gps: Time<Gps> = ...;
let utc: Time<Utc> = ...;

// ❌ compile error
let x = gps + utc;
```

## Design goals

- Prevent GNSS time domain mixing at compile time
- Make leap seconds explicit and unavoidable
- Provide deterministic conversions where possible
- Achieve zero-cost abstractions over raw timestamps
- Be fully `no_std` compatible
- Serve as a foundational GNSS time layer

## Supported scales

| Scale   | Format            |
| ------- | ----------------- |
| GLONASS | Day / TOD         |
| GPS     | Week / TOW        |
| Galileo | Week / TOW        |
| BeiDou  | Week / TOW        |
| TAI     | Seconds + nanos   |
| UTC     | Leap-second aware |

## Status

- [x] Core types
- [x] Fixed conversions
- [x] Leap second handling
- [x] Conversion matrix
- [x] Embedded-safe arithmetic

## License

Dual-licensed under:

- MIT
- Apache-2.0
