# Architecture

Internal design of `gnss-time`.

## Module layout

```text
src/
├── tables/
│   ├── leap_seconds.rs  — BUILTIN_TABLE (19 GPS-era entries)
│   └── mod.rs
├── convert.rs      — IntoScale / IntoScaleWith traits + all implementations
├── duration.rs     — Duration (signed interval in nanoseconds)
├── epoch.rs        — CivilDate, constant epoch offsets, Unix offsets
├── error.rs        — GnssTimeError
├── leap.rs         — LeapSecondsProvider, LeapSeconds, all conversion functions
├── lib.rs          — crate root, #![no_std], pub use re-exports
├── matrix.rs       — ConversionMatrix, ScaleId, ConversionKind
├── prelude.rs      — convenient re-exports
├── scale.rs        — sealed trait TimeScale + 6 marker types
├── serde_impls.rs  — Serialize/Deserialize for Time<S>, Duration, DurationParts
│                     (only when feature = "serde")
└── time.rs         — Time<S> struct, constructors, arithmetic, Unix methods
```

## Core invariant: TAI as the universal pivot

Any conversion with a fixed offset goes through TAI:

```text
T_tai = T_self + S::OFFSET_TO_TAI
T_target = T_tai - Target::OFFSET_TO_TAI
```

This means that all pairwise conversions are derived from a single consistent
set of offsets relative to TAI. There is no possibility of off-by-one errors
between individual pairs of scales.

The offsets (in nanoseconds) are compile-time constants, embedded in the enum
`OffsetToTai`:

| Scale   | OFFSET_TO_TAI      |
| ------- | ------------------ |
| GPS     | +19_000_000_000 ns |
| Galileo | +19_000_000_000 ns |
| BeiDou  | +33_000_000_000 ns |
| TAI     | 0                  |
| UTC     | Contextual         |
| GLONASS | Contextual         |

## Sealed trait pattern

`TimeScale` is a sealed trait — it cannot be implemented outside this crate:

```rust
mod private { pub trait Sealed {} }

pub trait TimeScale: private::Sealed + ... { ... }
```

This prevents a user from creating a new "pseudo time scale" that silently
breaks all conversions. The set of supported scales is fixed.

## Memory representation

`Time<S>` is exactly 8 bytes (identical to `u64`):

```rust
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,  // zero-sized
}
```

- The marker types `S` (`Gps`, `Glonass`, …) are also zero-sized
- No heap allocations
- All typing exists only at compile time

## Leap-second architecture

### Why explicit context?

```rust
// ❌ Hidden state — where do the leap seconds come from?
let utc = gps.to_utc();

// ✅ Explicit context — testable, no_std-compatible, deterministic
let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
```

### Two-pass UTC → GPS algorithm

A naive UTC → GPS conversion yields a ±1 second error near the moment of a
leap-second insertion. The library uses a two-pass algorithm:

**Pass 1:** TAI is computed approximately, assuming GPS − UTC = 0

**Pass 2:** refinement using the number of leap seconds from the first pass

This removes the error at the boundaries of all historical leap-second
insertions. The `utc_to_gps` tests cover all 18 transitions of the GPS era.

## Unix time interoperability

`Time<Utc>` counts nanoseconds from **1972-01-01** (UTC epoch), whereas Unix
time counts from **1970-01-01**. The difference is
`UTC_EPOCH_UNIX_OFFSET_S = 63_072_000 s` (730 days).

```text
unix_seconds    = utc_seconds_from_1972 + UTC_EPOCH_UNIX_OFFSET_S
utc_from_1972   = unix_seconds          - UTC_EPOCH_UNIX_OFFSET_S
```

Provided methods:

| Type         | Method                                       |
| ------------ | -------------------------------------------- |
| `Time<Utc>`  | `from_unix_seconds(i64) -> Result<Self>`     |
| `Time<Utc>`  | `from_unix_nanos(i64)   -> Result<Self>`     |
| `Time<Utc>`  | `as_unix_seconds() -> i64`                   |
| `Time<Utc>`  | `as_unix_nanos()   -> i64`                   |
| `Time<Gps>`  | `from_unix_seconds(i64, P) -> Result<Self>`  |
| `Time<Gps>`  | `as_unix_seconds(P) -> Result<i64>`          |

## Serde support (feature = "serde")

Enable it:

```toml
gnss-time = { version = "0.7", features = ["serde"] }
```

### Formats

#### `Time<S>`

**Human-readable** (JSON, TOML, YAML):

```json
{ "scale": "GPS", "nanos": 1356566418000000000 }
```

The `scale` field is validated during deserialization — trying to deserialize
`{ "scale": "UTC", ... }` into `Time<Gps>` returns an error.

**Compact** (postcard, bincode, MessagePack): a raw `u64` of nanoseconds with
no scale tag. The scale is carried by the type system.

#### `Duration`

| Format         | Form                        |
| -------------- | --------------------------- |
| Human-readable | `{ "nanos": -7000000000 }`  |
| Compact        | raw `i64`                   |

#### `DurationParts`

| Format         | Form                                  |
| -------------- | ------------------------------------- |
| Human-readable | `{ "seconds": 5, "nanos": 500000000 }`|
| Compact        | 2-element tuple `[u64, u32]`          |

### Implementation principles

- **No proc-macro** — implementations are written by hand using the `serde`
  visitor API
- **no_std compatible** — `serde` is pulled in with `default-features = false`
- `is_human_readable()` determines the format at runtime — one implementation
  works with both JSON and postcard
- Scale errors do not require `alloc` — `fmt::Display` is used

```rust
// Example — JSON round-trip
let gps = Time::<Gps>::from_seconds(1_356_566_418);
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

let back: Time<Gps> = serde_json::from_str(&json).unwrap();
assert_eq!(gps, back);

// Example — postcard round-trip
let bytes = postcard::to_allocvec(&gps).unwrap();
let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();
assert_eq!(gps, back);
```

## Feature flags

| Feature | Effect                                             |
| ------- | -------------------------------------------------- |
| (none)  | Pure `no_std`, no external dependencies            |
| `std`   | `impl std::error::Error for GnssTimeError`         |
| `serde` | `Serialize`/`Deserialize` for all public types     |
| `alloc` | Heap strings in serde error messages               |
| `defmt` | `impl defmt::Format` for all public types          |

## Conversion trait design

```rust
// Fixed offset — GPS ↔ TAI, GPS ↔ Galileo, GLONASS ↔ UTC
pub trait IntoScale<Target: TimeScale>: Sized {
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}

// Contextual conversions — GPS ↔ UTC, GPS ↔ GLONASS, etc.
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    fn into_scale_with<P: LeapSecondsProvider>(self, ls: P)
        -> Result<Time<Target>, GnssTimeError>;
    fn into_scale_with_checked<P: LeapSecondsProvider>(self, ls: P)
        -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}
```

`ConvertResult<T>` adds a signal about falling into a leap-second ambiguity
window.

## CI guarantees

| Check                              | Tool                                                            |
| ---------------------------------- | --------------------------------------------------------------- |
| No unsafe code                     | `#![forbid(unsafe_code)]`                                       |
| No undocumented API                | `#![deny(missing_docs)]`                                        |
| Builds for embedded targets        | `cargo check --target thumbv7em-none-eabihf`                    |
| Type size = 8 bytes                | unit test `test_size_equals_u64`                                |
| Safe arithmetic                    | `-D warnings` + absence of `#[allow(arithmetic_overflow)]`      |
| Serde roundtrip (JSON)             | tests in `src/serde_impls.rs`                                   |
| Serde roundtrip (postcard)         | tests in `src/serde_impls.rs`                                   |
