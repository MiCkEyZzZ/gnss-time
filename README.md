# gnss-time

[![CI](https://github.com/MiCkEyZzZ/gnss-time/actions/workflows/ci.yml/badge.svg)](https://github.com/MiCkEyZzZ/gnss-time/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/gnss-time.svg)](https://crates.io/crates/gnss-time)
[![docs.rs](https://docs.rs/gnss-time/badge.svg)](https://docs.rs/gnss-time)

**Strongly typed GNSS time model with explicit conversion semantics and zero-cost
arithmetic.**

`gnss-time` is a high-performance temporal abstraction layer for representing and
converting time across GNSS and atomic time scales. It models time as a **typed
multi-scale system**, not a single linear timeline.

Supported time scales:

- **GPS**, **GLONASS**, **Galileo**, **BeiDou**, **TAI**, **UTC**

This crate prioritizes:

- correctness over convenience
- explicitness over implicit conversions
- deterministic behavior over hidden state

It is **not** a navigation or positioning library.

## Quick start

```bash
cargo add gnss-time
```

```rust
use gnss_time::prelude::*;

let gps = Time::<Gps>::from_week_tow(
    2200,
    DurationParts { seconds: 0, nanos: 0 },
).unwrap();

// Fixed conversion (zero-cost)
let gal: Time<Galileo> = gps.into_scale().unwrap();

// Leap-second aware conversion
let result = gps.into_scale_with_checked(LeapSeconds::builtin()).unwrap();

match result {
    ConvertResult::Exact(utc) => println!("UTC: {utc}"),
    ConvertResult::AmbiguousLeapSecond(utc) => {
        println!("Leap second ambiguity: {utc}");
    }
}
```

## Installation

`gnss-time` is `no_std` by default and requires **no allocation**:

```toml
[dependencies]
gnss-time = "0.5"
```

For embedded targets nothing else is needed; optional integrations are enabled
via feature flags (see below).

## Feature flags

| Feature  | Default | Description                                                                 |
| -------- | ------- | --------------------------------------------------------------------------- |
| `std`    | no      | `impl std::error::Error` for `GnssTimeError` (do not enable for embedded)    |
| `serde`  | no      | `Serialize` / `Deserialize` for `Time<S>`, `Duration`, `DurationParts`       |
| `defmt`  | no      | `impl defmt::Format` for all public types (structured embedded logging)      |
| `alloc`  | no      | Heap-backed error messages in `serde` deserialization                        |

```toml
[dependencies]
gnss-time = { version = "0.5", features = ["serde"] }
```

## Usage

### Basic construction

```rust
use gnss_time::prelude::*;

let gps = Time::<Gps>::from_week_tow(
    2345,
    DurationParts { seconds: 432_000, nanos: 0 },
).unwrap();
```

### Safe arithmetic

`checked_*`/`saturating_*`/`try_*` variants never panic — prefer them in
embedded code:

```rust
let t = Time::<Gps>::from_week_tow(2345, DurationParts::new(432_000, 0).unwrap()).unwrap();

let safe: Option<Time<Gps>> = t.checked_add(Duration::from_seconds(3600));
let clamped: Time<Gps> = t.saturating_add(Duration::from_seconds(3600));
let fallible: Result<Time<Gps>, _> = t.try_add(Duration::from_seconds(3600));
```

### Leap-second aware conversion

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

### Civil time (ISO 8601 / RFC 3339)

```rust
use gnss_time::{Time, Utc};

let utc = Time::<Utc>::EPOCH;
let civil = utc.to_civil();

assert_eq!(
    civil.to_string(),
    "1972-01-01T00:00:00.000000000Z"
);
```

`CivilDateTime` is a proleptic Gregorian representation (year, month, day,
hour, minute, second, nanoseconds) with a lossless round-trip:

- `Time<Utc> ↔ CivilDateTime ↔ Time<Utc>` — exact nanoseconds preserved
- ISO 8601 / RFC 3339 formatting

## Time scale model

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

Conversions are classified as:

- **Fixed** → constant offset, zero-cost
- **EpochShift** → deterministic remapping
- **Contextual** → leap-second dependent (UTC only)

| Scale   | Representation    |
| ------- | ----------------- |
| GLONASS | Day / TOD         |
| GPS     | Week / TOW        |
| Galileo | Week / TOW        |
| BeiDou  | Week / TOW        |
| TAI     | seconds + nanos   |
| UTC     | leap-second aware |

### Three-layer architecture

```text
[ Arithmetic Layer ]   raw u64 nanoseconds, zero-cost operations
        ↓
[ GNSS Scale Layer ]   GPS / Galileo / BeiDou / GLONASS / TAI
        ↓
[ UTC / Civil Layer ]  leap-second aware, discontinuous, possibly non-invertible
```

### Type-safe domains

Each scale is a distinct type (`Gps`, `Glonass`, `Galileo`, `Beidou`, `Tai`,
`Utc`); cross-domain arithmetic is rejected at compile time:

```rust
// ❌ compile error — mixing scales is not allowed
gps + utc;
```

The library models the full conversion graph as a runtime-inspectable
`ConversionMatrix` (6×6, fixed vs contextual edges, `ScaleId`).

## Safety model

- **No domain mixing** — cross-scale operations are rejected at compile time.
- **Leap-second explicitness** — UTC is discontinuous, not globally invertible,
  and state-dependent; ambiguity is representable (`ConvertResult`), not hidden.
- **Determinism** — GNSS fixed conversions are deterministic; UTC conversions
  depend on the leap-second table; overflow behavior is explicit
  (`checked`/`saturating`/`try_*`, never silent).

## Performance

### Arithmetic

| Operation                      | Cost    |
| ------------------------------ | ------- |
| `Time + Duration` (panic path) | ~0.5 ns |
| `checked_add`                  | ~4.3 ns |
| `saturating_add`               | ~0.5 ns |

### Conversions

| Operation                    | Cost        |
| ---------------------------- | ----------- |
| GPS → TAI / Galileo / BeiDou | ~0.8–1.0 ns |
| GPS → UTC (leap-aware)       | ~9–10 ns    |
| UTC → GPS                    | ~22 ns      |
| Leap-second binary search    | ~6–7 ns     |

Round-trip `GPS → UTC → GPS`: ~37 ns, dominated by UTC context resolution.

All figures come from the `benches/` workspace crate (Criterion). Run locally:

```bash
just bench
```

See [`benches/README.md`](benches/README.md) for the latest result tables.

## Documentation

- [Embedded guide](docs/EMBEDDED.md) — `no_std`, size report, Postcard wire
  format, UBX/GLONASS parsing examples
- [Architecture](docs/ARCHITECTURE.md) — module layout, TAI pivot, feature flags
- [Leap seconds](docs/LEAP_SECONDS.md) — full table reference, update policy,
  IERS Bulletin C monitoring
- [Invariants](docs/INVARIANTS.md) — type-level, arithmetic, conversion and
  memory guarantees
- [GNSS time primer](docs/GNSS_TIME_PRIMER.md) — GPS/GLONASS/UTC/TAI explained
  for developers
- [Changelog](CHANGELOG.md)

## Minimum Supported Rust Version

**Rust 1.75.0** — enforced in CI.

## Contributing

Bugs, feature requests and pull requests are welcome. See the
[issue templates](.github/ISSUE_TEMPLATE/) and
[pull request template](.github/pull_request_template.md). Note the
[`CODEOWNERS`](.github/CODEOWNERS) file for review assignments and the semantic PR
title convention (`type(scope): description`).

## License

Licensed under either:

- [Apache License, Version 2.0](LICENSE.APACHE)
- [MIT License](LICENSE.MIT)
