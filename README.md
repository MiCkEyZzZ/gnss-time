# gnss-time

![Crates.io](https://img.shields.io/crates/v/gnss-time)
![no_std](https://img.shields.io/badge/no__std-yes-blue)
[![docs.rs](https://docs.rs/gnss-time/badge.svg)](https://docs.rs/gnss-time)
![MSRV](https://img.shields.io/badge/MSRV-1.75-blue)
![Embedded](https://img.shields.io/badge/embedded-friendly-green)

**Strongly typed GNSS time model with explicit conversion semantics and zero-cost
arithmetic.**

`gnss-time` is a high-performance temporal abstraction layer for representing and
converting time across GNSS and atomic time scales.

It models time as a **typed multi-scale system**, not a single linear timeline.

Supported time scales:

- GPS
- GLONASS
- Galileo
- BeiDou
- TAI
- UTC

This crate prioritizes:

- correctness over convenience
- explicitness over implicit conversions
- deterministic behavior over hidden state

It is not a navigation or positioning library.

## 1. System Model (Mental Model)

### 1.1 Time is not a single domain

Each GNSS time scale differs in:

- epoch origin
- unit definition
- discontinuities (leap seconds)

Therefore:

> Each time scale is a distinct type.

This prevents invalid mixing at compile time.

### 1.2 Three-layer time architecture

The system is structured as:

```text
[ Arithmetic Layer ]
    ↓
[ GNSS Scale Layer ]
    ↓
[ UTC / Civil Layer ]
```

#### Layer 1 — Arithmetic

- raw time representation (`u64 nanoseconds`)
- zero-cost operations

#### Layer 2 — GNSS scales

- GPS / Galileo / BeiDou / GLONASS / TAI
- fixed or epoch-shift conversions

#### Layer 3 — UTC / Civil time

- leap-second aware
- discontinuous timeline
- possibly non-invertible

## 1.3 Conversion semantics

Conversions are classified as:

- **Fixed** → constant offset, zero-cost
- **EpochShift** → deterministic remapping
- **Contextual** → leap-second dependent (UTC only)

## 2. Core Abstractions

### 2.1 Type-safe time domains

Each scale is a distinct type:

- `Gps`
- `Glonass`
- `Galileo`
- `Beidou`
- `Tai`
- `Utc`

Cross-domain arithmetic is **not allowed implicitly**.

```rust
// ❌ compile error
gps + utc;
```

### 2.2 Zero-cost arithmetic model

Arithmetic compiles down to native integer operations:

- `Time + Duration` ≈ `u64 + u64`
- no heap allocation
- no runtime dispatch

### 2.3 Explicit conversion graph

The library models a conversion graph:

- 6×6 scale matrix
- fixed vs contextual edges
- runtime inspectable structure

## 3. API Overview

## 3.1 Basic usage

```rust
use gnss_time::prelude::*;

let gps = Time::<Gps>::from_week_tow(
    2200,
    DurationParts { seconds: 0, nanos: 0 },
).unwrap();

// Fixed conversion (zero-cost)
let gal: Time<Galileo> = gps.into_scale().unwrap();
```

### 3.2 Leap-second aware conversion

UTC conversions require explicit handling:

```rust
use gnss_time::prelude::*;

let gps = Time::<Gps>::from_week_tow(
    2200,
    DurationParts { seconds: 0, nanos: 0 },
).unwrap();

let ls = LeapSeconds::builtin();

let result = gps.into_scale_with_checked(ls).unwrap();

match result {
    ConvertResult::Exact(utc) => {
        println!("UTC: {}", utc);
    }
    ConvertResult::AmbiguousLeapSecond(utc) => {
        println!("Leap second ambiguity: {}", utc);
    }
}
```

### 3.3 Civil time representation

```rust
use gnss_time::{Time, Utc};

let utc = Time::<Utc>::EPOCH;
let civil = utc.to_civil();

assert_eq!(
    civil.to_string(),
    "1972-01-01T00:00:00.000000000Z"
);
```

#### CivilDateTime

Proleptic Gregorian UTC representation:

- year, month, day
- hour, minute, second
- nanoseconds

#### Guarantees

- Lossless round-trip:
  - `Time<Utc> ↔ CivilDateTime ↔ Time<Utc>`

- ISO 8601 / RFC 3339 formatting
- nanosecond precision preserved

## 4. GNSS Time Model

GNSS systems define incompatible time scales:

| System  | Definition                     |
| ------- | ------------------------------ |
| GPS     | TAI − 19s                      |
| Galileo | TAI − 19s                      |
| BeiDou  | TAI − 33s                      |
| GLONASS | UTC(SU)-aligned                |
| TAI     | continuous atomic time         |
| UTC     | leap-second discontinuous time |

> A single physical instant may have multiple valid representations.

## 5. Safety Model

## 5.1 No domain mixing

Cross-scale operations are rejected at compile time.

### 5.2 Leap-second explicitness

UTC is:

- discontinuous
- not globally invertible
- state-dependent

This is modeled explicitly in:

```rust
ConvertResult
```

### 5.3 Determinism rules

- GNSS fixed conversions are deterministic
- UTC conversions depend on leap-second table
- ambiguous states are representable, not hidden

## 6. Performance Model

### 6.1 Arithmetic layer

| Operation                      | Cost    |
| ------------------------------ | ------- |
| `Time + Duration` (panic path) | ~0.5 ns |
| `checked_add`                  | ~4.3 ns |
| `saturating_add`               | ~0.5 ns |

### 6.2 Conversion layer

| Operation                    | Cost        |
| ---------------------------- | ----------- |
| GPS → TAI / Galileo / BeiDou | ~0.8–1.0 ns |
| GPS → UTC (leap-aware)       | ~9–10 ns    |
| UTC → GPS                    | ~22 ns      |
| Leap-second binary search    | ~6–7 ns     |

### 6.3 Round-trip behavior

- GPS → UTC → GPS: ~37 ns
- cost dominated by UTC context resolution

## 7. Design Constraints

### 7.1 UTC is contextual

UTC conversions:

- require leap-second table
- may be ambiguous
- are not always invertible

### 7.2 Fixed vs contextual boundary

Only UTC crosses the boundary:

```text
GNSS scales → deterministic algebra
UTC → stateful discontinuity system
```

### 7.3 No implicit coercions

All conversions must be explicit:

- prevents silent epoch mistakes
- enforces correctness at compile time

## 8. Supported Scales

| Scale   | Representation    |
| ------- | ----------------- |
| GLONASS | Day / TOD         |
| GPS     | Week / TOW        |
| Galileo | Week / TOW        |
| BeiDou  | Week / TOW        |
| TAI     | seconds + nanos   |
| UTC     | leap-second aware |

## 9. Status

- [x] Core time algebra
- [x] GNSS scale model
- [x] Fixed conversions
- [x] Contextual UTC handling
- [x] Leap-second engine
- [x] Conversion matrix inspection
- [x] Embedded-safe arithmetic
- [x] Civil datetime layer

## 10. Rust Version

Minimum Supported Rust Version (MSRV):

- **Rust 1.75.0**

Enforced in CI.

## 11. License

Licensed under either:

- [Apache License, Version 2.0](LICENSE.APACHE)
- [MIT License](LICENSE.MIT)
