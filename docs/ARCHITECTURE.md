# Architecture

Internal design of `gnss-time`.

## Table of contents

- [Layered architecture](#layered-architecture)
- [Module layout](#module-layout)
- [Module dependency diagram](#module-dependency-diagram)
- [Core invariant: TAI as the pivot for fixed-offset conversions](#core-invariant-tai-as-the-pivot-for-fixed-offset-conversions)
- [Two classes of conversion: Fixed vs. Contextual](#two-classes-of-conversion-fixed-vs-contextual)
- [The sealed trait pattern](#the-sealed-trait-pattern)
- [Memory representation](#memory-representation)
- [Leap-second architecture](#leap-second-architecture)
- [Unix time interoperability](#unix-time-interoperability)
- [Civil date-time (ISO 8601)](#civil-date-time-iso-8601)
- [Serde support](#serde-support-feature--serde)
- [Feature flags](#feature-flags)
- [Extending: adding a new time scale](#extending-adding-a-new-time-scale)
- [Limitations](#limitations)
- [CI guarantees](#ci-guarantees)

---

## Layered architecture

`gnss-time` is organized into five layers. The dependencies between layers
are acyclic: each layer only depends on the layers below it.

```text
┌───────────────────────────────────────────────────────────────┐
│  5. matrix         ConversionMatrix, ScaleId — runtime        │
│                    introspection of the conversion graph      │
├───────────────────────────────────────────────────────────────┤
│  4. convert        IntoScale, IntoScaleWith, ConvertResult —  │
│                    the public conversion API surface          │
├───────────────────────────────────────────────────────────────┤
│  3. leap           LeapSecondsProvider, LeapSeconds,          │
│                    RuntimeLeapSeconds — the contextual        │
│                    (leap-second-aware) conversion functions   │
├───────────────────────────────────────────────────────────────┤
│  2. time           Time<S> — the core value type, arithmetic, │
│                    fixed-offset conversions (to_tai/from_tai) │
├───────────────────────────────────────────────────────────────┤
│  1. scale + epoch  TimeScale trait, marker types (Gps, Utc,   │
│                    …), CivilDate, epoch offset constants      │
└───────────────────────────────────────────────────────────────┘
```

**Layer 1 (`scale`, `epoch`)** defines *what a time scale is*: a
zero-sized marker type plus a compile-time relationship to TAI. `epoch`
supplies the calendar arithmetic (`CivilDate`) used to derive those
relationships and is otherwise independent of `scale`.

**Layer 2 (`time`)** defines `Time<S>`, the single value type of the crate,
and everything that requires no external data: construction, arithmetic
(`+`, `-`, `checked_add`, …), and conversions between scales that share a
*fixed* offset to TAI (`to_tai`, `from_tai`, `try_convert`).

**Layer 3 (`leap`)** adds the contextual conversions — those that depend on
a leap-second table supplied at the call site (`gps_to_utc`, `utc_to_gps`,
and the GLONASS/Galileo/BeiDou variants derived from them). This layer
introduces the `LeapSecondsProvider` trait and its two concrete
implementations (plus a blanket `&P` impl).

**Layer 4 (`convert`)** is the public, ergonomic front door:
`IntoScale`/`IntoScaleWith` wrap the layer-2/layer-3 functions behind a
uniform trait-based API and add `ConvertResult` for leap-second ambiguity
reporting.

**Layer 5 (`matrix`)** is documentation-as-code: its primary job is to
classify every pair of scales
(`Fixed`/`Identity`/`EpochShift`/`Contextual`/`SameScale`) for runtime
introspection and for the exhaustive pairwise tests that keep this document
honest. It performs no conversions itself, with a single exception:
`beidou_via_gps_to_glonass_via_utc` is a real conversion chain kept here so
that `ConversionChain` can carry the GPS/UTC/TAI intermediate values that
the introspection example needs.

Two modules sit outside this stack because they are optional or purely
additive:

- **`civil`** adds a human-readable calendar view (`CivilDateTime`) on top of
  `Time<Utc>`; it does not participate in any conversion.
- **`serde_impls`** (feature-gated) adds `Serialize`/`Deserialize`; it has no
  conversion logic of its own.

## Module layout

```text
src/
├── tables/
│   ├── leap_seconds.rs  — BUILTIN_TABLE (19 GPS-era entries)
│   └── mod.rs
├── scale.rs         — layer 1: sealed trait TimeScale + 6 marker types
├── epoch.rs         — layer 1: CivilDate, constant epoch offsets, Unix offsets
├── time.rs          — layer 2: Time<S> struct, constructors, arithmetic,
│                      fixed-offset conversion (to_tai/from_tai), Unix methods
├── leap.rs          — layer 3: LeapSecondsProvider, LeapSeconds,
│                      RuntimeLeapSeconds, all contextual conversion functions
├── convert.rs       — layer 4: IntoScale / IntoScaleWith traits + all
│                      per-scale-pair implementations, ConvertResult
├── matrix.rs        — layer 5: ConversionMatrix, ScaleId, ConversionKind
├── civil.rs         — CivilDateTime (ISO 8601 / RFC 3339, from Time<Utc>)
├── error.rs         — GnssTimeError (used by every layer)
├── serde_impls.rs   — Serialize/Deserialize for Time<S>, Duration,
│                      DurationParts (only when feature = "serde")
├── duration.rs      — Duration (signed interval in nanoseconds; no
│                      dependency on scale/time — pure arithmetic type)
├── prelude.rs       — convenient re-exports
└── lib.rs           — crate root, #![no_std], pub use re-exports
```

`duration.rs` is not part of the five-layer stack above: `Duration`
represents an *interval*, not a point in time, and has no relationship to
`TimeScale` at all. It is used by layer 2 (`Time<S>` arithmetic) but does not
depend on it.

## Module dependency diagram

```text
                        ┌───────────┐
                        │  duration │  (no crate-internal deps besides error)
                        └─────┬─────┘
                              │
 ┌───────────┐   ┌───────────┐│┌───────────┐
 │   epoch   │──▶│   scale   │┴│   error   │  (leaves — no crate-internal deps
 └───────────┘   └─────┬─────┘ └─────┬─────┘   besides each other's absence)
                       │             │
                       ▼             │
                 ┌───────────┐       │
                 │    time   │◀──────┘
                 └─────┬─────┘
                       │
                       ▼
            ┌───────────┐   ┌───────────┐
            │   tables  │──▶│    leap   │
            └───────────┘   └─────┬─────┘
                                  │
                                  ▼
                            ┌───────────┐
                            │  convert  │
                            └─────┬─────┘
                                  │
                                  ▼
                            ┌───────────┐
                            │   matrix  │
                            └───────────┘

            ┌───────────┐                  ┌─────────────────┐
  time ────▶│   civil   │◀──── time        │  serde_impls    │────▶ scale
            └───────────┘                  │ (feature=serde) │────▶ time
                                           └─────────────────┘
            (civil does not participate in any conversion)
```

Arrows mean "depends on". `error` is depended on by every layer (all
fallible operations return `GnssTimeError`) and is omitted from the arrows
above for readability, except where it is a leaf itself.

Note the `time` ↔ `civil` cycle: `Time<Utc>` has a
`to_civil()` convenience that returns `CivilDateTime`, and `civil` builds
`CivilDateTime` from `Time<Utc>`. Both modules reference each other. This is
a module-level cycle inside the crate (legal in Rust) that exists purely for
the calendar-view convenience; it is not a conversion dependency.

## Core invariant: TAI as the pivot for fixed-offset conversions

Any conversion between two scales that both have a **fixed** offset to TAI
goes through TAI as an intermediate value:

```text
T_tai    = T_self   + S::OFFSET_TO_TAI
T_target = T_tai     - Target::OFFSET_TO_TAI
```

Concretely, this is `Time::<S>::to_tai` followed by `Time::<Target>::from_tai`,
composed as `Time::<S>::try_convert::<Target>()`. This means that **all**
pairwise conversions between fixed-offset scales are derived from a single
consistent set of offsets relative to TAI — there is no possibility of
off-by-one errors between individual pairs of scales, because no pair is
special-cased. GPS ↔ Galileo, GPS ↔ BeiDou, GPS ↔ TAI, Galileo ↔ BeiDou, and
Galileo/BeiDou ↔ TAI are all the *same* two-line composition with different
constants.

The two contextual scales — UTC and GLONASS — use `OffsetToTai::Contextual`,
so they have no constant TAI relation and cannot participate in
`try_convert`. GLONASS ↔ UTC is nevertheless still a **fixed** conversion at
the `IntoScale` level (see below): GLONASS is defined from UTC(SU) =
UTC + 3 h, a constant epoch shift that does not go through TAI at all.

The offsets (in nanoseconds) are compile-time constants embedded in the enum
`OffsetToTai`:

| Scale   | `OFFSET_TO_TAI`             |
| ------- | --------------------------- |
| GPS     | `Fixed(+19 000 000 000)` ns |
| Galileo | `Fixed(+19 000 000 000)` ns |
| BeiDou  | `Fixed(+33 000 000 000)` ns |
| TAI     | `Fixed(0)`                  |
| UTC     | `Contextual`                |
| GLONASS | `Contextual`                |

GPS and Galileo sharing the identical fixed offset is what makes their
conversion an **identity** on the underlying nanosecond count (see
`ConversionKind::Identity` in `matrix.rs`) — no arithmetic at all is needed,
only a change of the phantom type parameter.

## Two classes of conversion: Fixed vs. Contextual

Every ordered pair of scales falls into exactly one of five kinds, classified
by `ScaleId::conversion_kind` in `matrix.rs`:

| Kind         | Requires external data?           | Trait           | Example                       |
| ------------ | --------------------------------- | --------------- | ----------------------------- |
| `SameScale`  | No                                | n/a             | `Gps → Gps`                   |
| `Identity`   | No                                | `IntoScale`     | `Gps ↔ Galileo`               |
| `Fixed`      | No                                | `IntoScale`     | `Gps ↔ Tai`, `Gps ↔ Beidou`   |
| `EpochShift` | No                                | `IntoScale`     | `Glonass ↔ Utc`               |
| `Contextual` | **Yes** — a `LeapSecondsProvider` | `IntoScaleWith` | `Gps ↔ Utc`, `Gps ↔ Glonass`  |

**Fixed conversions** (`Identity`, `Fixed`, `EpochShift` — collectively,
`ScaleId::is_fixed`) are pure functions of their input: no leap-second table,
no fallible external lookup, only overflow can fail them. They implement
[`IntoScale`], whose signature carries no such dependency:

```rust
pub trait IntoScale<Target: TimeScale>: Sized {
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}
```

**Contextual conversions** require knowing the current TAI − UTC offset,
which is not a compile-time constant — it changes every time IERS
schedules a leap second. Hiding this dependency behind global mutable state
would break `no_std` support, make testing require mocks, and make the
result depend on *when* the crate was compiled rather than on explicit,
inspectable input. Instead, every contextual conversion takes a
`LeapSecondsProvider` explicitly:

```rust
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    fn into_scale_with<P: LeapSecondsProvider>(self, ls: P)
        -> Result<Time<Target>, GnssTimeError>;
    fn into_scale_with_checked<P: LeapSecondsProvider>(self, ls: P)
        -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}
```

```rust
// ❌ Hidden state — where do the leap seconds come from?
let utc = gps.to_utc();

// ✅ Explicit context — testable, no_std-compatible, deterministic
let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
```

`into_scale_with_checked` additionally reports whether the result falls
inside the one-second window around a leap-second insertion, via
`ConvertResult<T>`:

```rust
pub enum ConvertResult<T> {
    Exact(T),
    AmbiguousLeapSecond(T),
}
```

Outside that window every contextual conversion round-trips exactly
(`A → B → A == A`); inside it, the mapping from GPS/Galileo/BeiDou time to
UTC is not injective (two consecutive GPS seconds can map to the same UTC
civil second), so `Exact` cannot be guaranteed and `into_scale_with_checked`
exists precisely so callers can detect and handle that case instead of
silently trusting an approximate result.

### The two-pass UTC → GPS algorithm

A naive UTC → GPS conversion yields a ±1 second error near the moment of a
leap-second insertion, because the correction to apply depends on which side
of the insertion the *result* falls on — which is exactly what you don't
know yet. The library resolves this with a two-pass algorithm in
`utc_to_gps`:

**Pass 1:** TAI is computed approximately, assuming GPS − UTC = 0.

**Pass 2:** the leap-second count implied by pass 1 is used to refine the
result.

This removes the error at the boundaries of all 18 historical leap-second
insertions covered by the built-in table; the test suite and the
`fuzz_gps_utc`/`fuzz_utc_to_gps` fuzz targets exercise every one of them.

## The sealed trait pattern

`TimeScale` is a sealed trait — it cannot be implemented outside this crate:

```rust
mod private { pub trait Sealed {} }

pub trait TimeScale: private::Sealed + Copy + Clone + Eq + PartialEq + Debug {
    const NAME: &'static str;
    const OFFSET_TO_TAI: OffsetToTai;
    const EPOCH_CIVIL: CivilDate;
    const DISPLAY_STYLE: DisplayStyle;
}
```

`private::Sealed` is implemented only for the crate's own six marker types
(`Gps`, `Glonass`, `Galileo`, `Beidou`, `Tai`, `Utc`), and since `Sealed` is
declared in a private module, no downstream crate can name it — and
therefore cannot write `impl TimeScale for MyScale`.

This is a deliberate closed-world design: every `TimeScale` implementor must
correctly state its `OFFSET_TO_TAI` and fit into the conversion graph in
`convert.rs` and `matrix.rs`. If external types could implement the trait,
`try_convert::<T>()` would have to handle scales whose offset relationship
the crate cannot verify, and the "all pairwise conversions share one
consistent TAI pivot" invariant above would no longer be checkable by the
crate's own test suite. See [Extending](#extending-adding-a-new-time-scale)
for what adding a *new, crate-maintained* scale actually involves.

## Memory representation

`Time<S>` is exactly 8 bytes, identical to a bare `u64`:

```rust
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,   // zero-sized
}
```

- The marker types `S` (`Gps`, `Glonass`, …) are zero-sized.
- No heap allocations anywhere in the crate's own code (without the `serde`
  feature; `serde_impls` itself does not allocate either — see below).
- All type-level scale-checking exists only at compile time; nothing is
  stored at runtime beyond the eight bytes of `nanos`.

This is verified by `test_size_equals_u64` in `time.rs`'s test module and by
the `firmware/` size-probe crate (`docs/EMBEDDED.md`), which measures the
compiled `.text` size of representative operations on `thumbv7em-none-eabihf`.

## Leap-second architecture

The built-in leap-second table holds 19 entries: the initial state of the
GPS era (TAI − UTC = 19 s at the GPS epoch, 1980-01-06) plus 18 subsequent
leap-second insertions, ending at TAI − UTC = 37 s (2017-01-01, the most
recent one as of this writing). `tables/leap_seconds.rs` embeds compile-time
assertions (`const` blocks) that the table is strictly sorted by threshold
and that every entry increments the offset by exactly 1 — a malformed
built-in table fails `cargo build`, not just `cargo test`.

Three implementations of `LeapSecondsProvider` exist:

- **`LeapSeconds`** — wraps the static `&'static [LeapEntry]` built-in
  table (or any other `'static` slice via `from_table`/`try_from_slice`).
  Zero-cost, no runtime mutation.
- **`RuntimeLeapSeconds`** — a fixed-capacity (`RUNTIME_CAPACITY = 64`),
  heap-free buffer that can be grown at runtime via `try_extend`, for
  receivers that learn new leap-second announcements from a navigation
  message. `try_extend` and `try_from_slice`/`from_slice` all enforce the
  same ordering and unit-increment invariants as the compile-time table —
  see `fuzz_try_extend.rs` for the fuzz harness that pins this contract,
  including the `OffsetOverflow` case found and fixed during that audit
  (§ [Fuzzing](../fuzz/README.md)).
- **`&P`** — a blanket `impl<P: LeapSecondsProvider> LeapSecondsProvider
  for &P`, so a reference to any of the above can be passed where a provider
  argument is expected (e.g. `gps_to_utc(t, &ls)`).

## Unix time interoperability

`Time<Utc>` counts nanoseconds from **1972-01-01** (the UTC epoch), whereas
Unix time counts from **1970-01-01**. The difference is
`UTC_EPOCH_UNIX_OFFSET_S = 63_072_000 s` (730 days):

```text
unix_seconds    = utc_seconds_from_1972 + UTC_EPOCH_UNIX_OFFSET_S
utc_from_1972   = unix_seconds          - UTC_EPOCH_UNIX_OFFSET_S
```

This is a pure count-to-count mapping — `Time<Utc>` stores a linear count of
nanoseconds with no leap-second discontinuities of its own. Leap seconds are
applied only when *converting between time scales* (the previous section);
the Unix mapping works purely on the internal linear representation and
never touches a `LeapSecondsProvider`.

| Type        | Method                                      |
| ----------- | ------------------------------------------- |
| `Time<Utc>` | `from_unix_seconds(i64) -> Result<Self>`    |
| `Time<Utc>` | `from_unix_nanos(i64) -> Result<Self>`      |
| `Time<Utc>` | `as_unix_seconds() -> i64`                  |
| `Time<Utc>` | `as_unix_nanos() -> i64`                    |
| `Time<Gps>` | `from_unix_seconds(i64, P) -> Result<Self>` |
| `Time<Gps>` | `as_unix_seconds(P) -> Result<i64>`         |

The `Time<Gps>` variants require a `LeapSecondsProvider` because they route
through `Time<Utc>` and then a contextual GPS↔UTC conversion; the
`Time<Utc>` variants do not, because they never leave the UTC scale.

## Civil date-time (ISO 8601)

`civil::CivilDateTime` is a human-readable view derived from `Time<Utc>`:
year/month/day/hour/minute/second/nanosecond fields plus a `Display`
implementation producing RFC 3339 / ISO 8601 text
(`2024-01-15T12:34:56.123456789Z`). It depends only on `Time<Utc>` and does
not participate in any scale conversion — `to_civil()` /
`CivilDateTime::to_utc()` are a lossless, allocation-free round trip that
never touches a leap-second table, because `Time<Utc>`'s own nanosecond
count already has none of the leap-second discontinuities that a wall-clock
calendar display would otherwise need to account for.

As noted in the [dependency diagram](#module-dependency-diagram), the
relationship between `time` and `civil` is bidirectional at the module level:
`Time<Utc>` gains the `to_civil()` convenience, and `civil` builds on
`Time<Utc>`.

## Serde support (`feature = "serde"`)

```toml
gnss-time = { version = "0.9", features = ["serde"] }
```

### Formats

`Time<S>` — **human-readable** (JSON, TOML, YAML):

```json
{ "scale": "GPS", "nanos": 1356566418000000000 }
```

The `scale` field is validated during deserialization: deserializing
`{ "scale": "UTC", … }` into `Time<Gps>` returns an error.

`Time<S>` — **compact** (postcard, bincode, MessagePack): a raw `u64` of
nanoseconds with no scale tag — the scale is carried entirely by the Rust
type, so there is nothing to validate or corrupt at this layer.

`Duration` and `DurationParts` follow the same human-readable/compact split:

| `Duration`     | Form                          |
| -------------- | ----------------------------- |
| Human-readable | `{ "nanos": -7000000000 }`    |
| Compact        | raw `i64`                     |

| `DurationParts`     | Form                                   |
| ------------------- | -------------------------------------- |
| Human-readable      | `{ "seconds": 5, "nanos": 500000000 }` |
| Compact             | 2-element tuple `[u64, u32]`           |

`DurationParts` is a separate, non-negative parts type (`seconds: u64`,
`nanos: u32`) used by the GNSS week/day constructors — it never encodes a
sign; negative intervals exist only in `Duration` itself. See
`docs/EMBEDDED.md` for the exact wire format and byte-size table.

### Implementation principles

- **No proc-macro** — implementations are hand-written against the `serde`
  visitor API, so the crate's `no_std` guarantee does not depend on
  `serde_derive`'s code-generation assumptions.
- **`no_std` compatible** — `serde` is pulled in with
  `default-features = false`.
- `is_human_readable()` selects the format at runtime, so one
  implementation serves both JSON and postcard.
- Scale-mismatch errors do not require `alloc` — they are produced via
  `fmt::Display`, not `String` formatting.

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

Note: the `gnss-time` serde code itself stays `no_std`-compatible; the
`postcard::to_allocvec()` call in the example additionally requires `alloc`
(the `alloc` feature of the `postcard` crate).

## Feature flags

| Feature | Effect                                                      |
| ------- | ----------------------------------------------------------- |
| (none)  | Pure `no_std`, zero external dependencies                   |
| `std`   | `impl std::error::Error for GnssTimeError`                  |
| `serde` | `Serialize`/`Deserialize` for all public types              |
| `alloc` | Reserved no-op — heap-backed serde error messages planned   |
| `defmt` | `impl defmt::Format` for all public types                   |

## Extending: adding a new time scale

Adding a scale means adding a new crate-internal implementor of the sealed
`TimeScale` trait via the `define_scale!` macro in `scale.rs`, then wiring it
into the conversion graph. Concretely, for a scale whose relationship to TAI
is a **fixed** offset (the common case — see
[QZSS/NavIC](../docs/GNSS_TIME_PRIMER.md) as a worked example, roadmap
item #TIME-32):

1. **`scale.rs`** — add a `define_scale!` invocation supplying `NAME`,
   `OFFSET_TO_TAI` (`Fixed(offset_ns)` or `Contextual`), `EPOCH_CIVIL`, and
   `DISPLAY_STYLE`.
2. **`convert.rs`** — implement `IntoScale<NewScale>` for every existing
   *fixed*-offset scale it should convert to/from directly (or rely on the
   blanket `try_convert::<T>()` composition through TAI, if a direct,
   documented `impl` isn't required). If the new scale is contextual instead
   (like UTC/GLONASS), implement `IntoScaleWith<NewScale>` and route through
   an existing contextual conversion the same way `Galileo`/`Beidou`→`Utc`
   route through `Gps`→`Utc` (see the `#TIME-27.1` audit fix in
   `CHANGELOG.md` for why routing through a single ambiguity-aware path
   matters, rather than re-deriving the leap-second logic per scale).
3. **`matrix.rs`** — add the new `ScaleId` variant, extend `ScaleId::ALL`,
   and extend `conversion_kind`'s match so every pair involving the new
   scale is classified. `ConversionMatrix::path_count` and the exhaustive
   pairwise tests in `matrix.rs` will then automatically include it.
4. **Tests** — add round-trip tests (`new_scale → existing_scale →
   new_scale` preserves nanoseconds for identity/fixed relationships) and,
   for a contextual scale, an ambiguity-window test mirroring
   `fuzz_gps_utc`/`fuzz_utc_to_gps`.
5. **Docs** — add a row to the offset table in
   [Core invariant](#core-invariant-tai-as-the-pivot-for-fixed-offset-conversions)
   and to `docs/GNSS_TIME_PRIMER.md`.

Because `TimeScale` is sealed, this list is exhaustive by construction: a
scale that skips step 3 simply won't appear in `ConversionMatrix`, and the
matrix's own pairwise-completeness tests will not catch a *missing* scale
(they iterate `ScaleId::ALL`, so an unlisted scale is invisible to them) —
which is precisely why step 3 has to be a deliberate, reviewed addition
rather than something the type system enforces automatically.

## Limitations

**Why is there no `From<Time<Gps>> for Time<Utc>`?**

`From`/`Into` in Rust are documented as *infallible* and are conventionally
expected to be cheap, "no way to fail" conversions. GPS→UTC is neither: it
requires an explicit `LeapSecondsProvider` (there is no default one the
trait could reach for without reintroducing the hidden global state this
crate deliberately avoids — see
[Fixed vs. Contextual](#two-classes-of-conversion-fixed-vs-contextual)
above), and even given a provider, the result can be `Overflow` for values
outside `Time`'s representable range, or ambiguous inside a leap-second
window. None of that fits `From`'s contract, so the crate exposes
`IntoScaleWith`/`gps_to_utc` instead, which make both the required context
and the possibility of failure explicit in the signature.

Even the **fixed**-offset conversions (`Gps → Tai`, `Gps → Galileo`, …) use
`IntoScale` rather than `TryFrom`, for a narrower reason: `TryFrom` would
work for any *one* fixed pair, but the crate wants exactly one trait whose
`impl`s are enumerable via `ScaleId`/`ConversionMatrix` for the
introspection story in layer 5 — mixing `TryFrom` (fixed) and a custom trait
(contextual) would fragment that story into "check `TryFrom` for this pair,
check `IntoScaleWith` for that one," with no single source of truth for
"which conversions exist." `IntoScale`/`IntoScaleWith` give both classes a
uniform shape that `matrix.rs` can describe exhaustively.

**Other limitations, by design:**

- **No `PartialOrd`/arithmetic across scales.** `Time<Gps> - Time<Glonass>`
  does not compile — comparing or subtracting across scales without an
  explicit conversion is exactly the class of bug the phantom-type design
  exists to prevent at compile time.
- **No leap-second table auto-update.** The built-in table is frozen at
  build time; a receiver that needs the latest IERS announcements must
  supply its own `RuntimeLeapSeconds` (see `docs/LEAP_SECONDS.md` for the
  update policy this implies for the crate maintainers, and
  `fuzz_try_extend.rs`/`fuzz_leap_lookup.rs` for the contract that runtime
  table must satisfy).
- **Leap seconds beyond the table's last entry are assumed constant.**
  `tai_minus_utc_at` returns the last known offset for any TAI instant past
  the final table entry, per IERS convention (no leap second is inserted
  without ≥6 months' notice) — this is a documented assumption, not a bug,
  but it does mean conversions for dates far in the future silently assume
  "no further leap seconds" rather than erroring.
- **No calendar arithmetic on `Time<S>` for GNSS scales.** Only `Time<Utc>`
  has a `to_civil()`/`CivilDateTime` view; GPS/Galileo/BeiDou/GLONASS
  timestamps must be converted to UTC first if a wall-clock date is needed,
  which is again an explicit, fallible step rather than an implicit one.

## CI guarantees

| Check                            | Tool                                                                                |
| -------------------------------- | ----------------------------------------------------------------------------------- |
| No unsafe code                   | `#![forbid(unsafe_code)]`                                                           |
| No undocumented API              | `#![deny(missing_docs)]`                                                            |
| Builds for embedded targets      | `cargo check --target thumbv7em-none-eabihf` (+ 4 more, see `docs/EMBEDDED.md`)     |
| Type size = 8 bytes              | unit test `test_size_equals_u64`                                                    |
| Safe arithmetic                  | `-D warnings` + absence of `#[allow(arithmetic_overflow)]`                          |
| Serde round-trip (JSON)          | tests in `src/serde_impls.rs`                                                       |
| Serde round-trip (postcard)      | `tests/serde_test.rs`                                                               |
| Leap-second table well-formed    | `const` assertions in `tables/leap_seconds.rs`                                      |
| Conversion graph completeness    | exhaustive pairwise tests in `matrix.rs`                                            |
| Conversion & lookup invariants   | property tests (`tests/prop_*.rs`) + fuzz harnesses (`fuzz/`, see `fuzz/README.md`) |
