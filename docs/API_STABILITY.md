# API stability

This document is the single source of truth for what is safe to depend on in
`gnss-time` today, what may still change before `1.0`, and what should never
be named directly. It exists because `#[non_exhaustive]` and doc comments
communicate stability *per item*, but nothing previously stated the policy
*as a whole* or listed every public item against it.

## Table of contents

- [Semver policy for `0.x`](#semver-policy-for-0x)
- [Stability legend](#stability-legend)
- [Audit: `#[non_exhaustive]` coverage](#audit-non_exhaustive-coverage)
- [API matrix](#api-matrix)
- [`pub use` re-export audit](#pub-use-re-export-audit)
- [Internal items that should be `#[doc(hidden)]`](#internal-items-that-should-be-dochidden)
- [Doc-test coverage](#doc-test-coverage)
- [CI enforcement](#ci-enforcement)

---

## Semver policy for `0.x`

`gnss-time` is pre-1.0 (`0.x`). Cargo's semver-compatibility rule for `0.x`
treats **the minor version as the breaking-change boundary**: `0.5.2` →
`0.6.0` may break; `0.5.1` → `0.5.2` must not. This crate follows that rule
strictly, with one addition specific to how the crate is built:

| Change                                                                                       | `0.x` bump                                                                                                                                                                             |
| -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New variant added to a `#[non_exhaustive]` enum                                              | **patch**                                                                                                                                                                              |
| New method added to an existing type                                                         | **patch**                                                                                                                                                                              |
| New field added to a `#[non_exhaustive]` struct                                              | **patch**                                                                                                                                                                              |
| New trait `impl` for an existing type                                                        | **patch**                                                                                                                                                                              |
| Deprecating an item (`#[deprecated]`, item kept and still works)                             | **patch**                                                                                                                                                                              |
| New `#[non_exhaustive]` public type or free function                                         | **minor**                                                                                                                                                                              |
| New builtin leap-second table entry (`tables/leap_seconds.rs`)                               | **patch** — see rationale below                                                                                                                                                        |
| Removing a `#[deprecated]` item                                                              | **minor**                                                                                                                                                                              |
| Changing a public function's signature (including its error type)                            | **minor**                                                                                                                                                                              |
| Changing `Display` output format for any public type                                         | **minor**                                                                                                                                                                              |
| Adding a variant to a **non**-`#[non_exhaustive]` enum                                       | **minor** (would be a breaking match elsewhere otherwise)                                                                                                                              |
| Widening a numeric parameter type (e.g. `u16` → `u32`, as in `#TIME-27.1`'s `from_week_tow`) | **minor** — call sites can break even though the change is "more permissive," because type inference and overload-like generic bounds can select differently                           |
| Narrowing a numeric parameter type, or changing a return type                                | **major** (or minor pre-1.0, but treated with major-level scrutiny)                                                                                                                    |
| Changing the `TimeScale::OFFSET_TO_TAI` constant for an existing scale                       | **major** — this is a correctness constant, not an API shape; changing it silently changes every downstream conversion's *numeric result*, which is a worse break than a compile error |

**Why is a new leap-second table entry a *patch*, not a minor bump?** A
leap-second insertion is IERS-announced, external, factual data — it does
not change any function's signature, error type, or the set of types a
caller can name. It only changes the *numeric output* of
`gps_to_utc`/`utc_to_gps` for instants past the new entry, in exactly the way
the crate's own documentation says it will (see
`docs/LEAP_SECONDS.md`'s update policy). Treating it as a patch means users
get corrected leap-second behavior via a normal `cargo update`, which is the
intended behavior — the alternative (treating every leap second as a minor
bump) would train users to pin `gnss-time = "=0.5.2"` and silently run stale
leap-second data indefinitely.

**Why is widening a parameter type flagged specially?** `#TIME-27.1` changed
`Time::<Gps>::from_week_tow`'s `week` parameter from `u16` to `u32` to match
`week() -> u32`. This is *usually* safe for direct callers (`u16` values
still convert), but it is not universally non-breaking: a caller that wrote
`from_week_tow(2345u16, …)` still compiles (integer literals are untyped),
but a caller that had `let w: u16 = ...; from_week_tow(w, ...)` now needs an
explicit `w as u32` (or `.into()`), and any call passed through a generic
`impl Into<u32>`-style wrapper changes its inferred type. There is no
`from_week_tow_u16` compatibility alias (an earlier draft of this document
claimed one); the widening is an out-and-out breaking change recorded under
`[0.7.0]` in `CHANGELOG.md`.

**Target `1.0` criteria** (not yet met): every item below marked
**Stable — pending 1.0** has passed a full `0.x` cycle with no reported
signature or semantic changes, the fuzz suite (`fuzz/README.md`) has run
continuously in CI with zero crashes for at least one release cycle, and
`docs/INVARIANTS.md`'s invariants are all cross-referenced to a passing test.

---

## Stability legend

| Badge                       | Meaning                                                                                                                                                                                                 |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 🟢 **Stable**               | Safe to depend on today. Will only change via the `0.x` rules above (i.e. treated as if it were post-1.0 for signature/semantics; may still gain `#[non_exhaustive]` members per the patch-bump table). |
| 🟡 **Stable — pending 1.0** | Design is settled and exercised by fuzzing/property tests, but has not yet completed a full release cycle unchanged. Expected to become 🟢 verbatim at `1.0`.                                           |
| 🟠 **Unstable**             | Shape may still change. Usable, but pin a specific `0.x.y` if you depend on its exact signature.                                                                                                        |
| 🔴 **Internal**             | Not part of the public contract even though currently reachable. Do not depend on it; see [`#[doc(hidden)]` recommendations](#internal-items-that-should-be-dochidden).                                 |

---

## Audit: `#[non_exhaustive]` coverage

Every public enum and every public struct intended for future field growth
was reviewed. Result:

| Type                 | `#[non_exhaustive]`?                       | Verdict                                                                                                                                                                             |
| -------------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GnssTimeError`      | ✅ yes                                     | Correct — new error variants are a patch-level addition.                                                                                                                            |
| `LeapExtendError`    | ✅ yes                                     | Correct — `try_extend`'s error set may grow (e.g. a future `OffsetOverflow`-style variant found by fuzzing, `CHANGELOG.md`).                                                        |
| `ConvertResult<T>`   | ✅ yes                                     | Correct — currently `Exact`/`AmbiguousLeapSecond`; a third variant (e.g. distinguishing which side of the leap second) is conceivable.                                              |
| `ConversionKind`     | ✅ yes                                     | Correct — `matrix.rs`'s classification enum; a new kind would be needed if a scale with a genuinely new relationship were added.                                                    |
| `ScaleId`            | ✅ yes                                     | Correct — enum listing the six scales for runtime introspection; adding a scale adds a variant, patch-safe via the attribute.                                                       |
| `DisplayStyle`       | ✅ yes                                     | Correct — internal to `scale.rs`'s `TimeScale::DISPLAY_STYLE` associated const; new display styles are plausible (e.g. a `CivilLike` style if another scale grows a calendar view). |
| `OffsetToTai`        | ✅ yes                                     | Correct — currently `Fixed(i64)`/`Contextual`; matches the Fixed/Contextual split in `docs/ARCHITECTURE.md`.                                                                        |
| `DurationParts`      | ❌ no (plain struct, 2 public fields)      | **Intentional exception** — see below.                                                                                                                                              |
| `CivilDateTime`      | ❌ no (plain struct, 7 public fields)      | **Intentional exception** — see below.                                                                                                                                              |
| `Time<S>`            | n/a (single private field + `PhantomData`) | Correct — no public fields to begin with; all access is via methods.                                                                                                                |
| `Duration`           | n/a (single private field)                 | Correct — same reasoning.                                                                                                                                                           |
| `LeapEntry`          | ❌ no (plain struct, 2 public fields)      | **Flagged — see recommendation below.**                                                                                                                                             |
| `LeapSeconds`        | n/a (opaque, private field(s))             | Correct.                                                                                                                                                                            |
| `RuntimeLeapSeconds` | n/a (opaque, private field(s))             | Correct.                                                                                                                                                                            |

**`DurationParts` and `CivilDateTime` are deliberately *not*
`#[non_exhaustive]`.** Both are plain data-carrying structs whose entire
purpose is to be constructed with a struct literal
(`DurationParts { seconds, nanos }`, `CivilDateTime { year, month, … }`) —
marking them `#[non_exhaustive]` would force every caller to use `::new()`
or `Default`-then-mutate, defeating the ergonomic point of the type. Adding
a field to either in the future would be a **minor** bump under the table
above, same as any breaking struct-literal change; this is accepted as the
cost of keeping literal construction available.

**`LeapEntry` is flagged for a `0.x` minor-version fix**: it is a plain
2-field struct (`tai_nanos: u64, tai_minus_utc: i32`) constructed directly by
callers building a custom leap-second table for `RuntimeLeapSeconds`/
`LeapSeconds::try_from_slice`. Unlike `DurationParts`/`CivilDateTime`, there
is no strong ergonomic reason to prefer a literal here — `LeapEntry::new(tai_nanos,
tai_minus_utc)` is exactly as readable as the literal and every callsite in
the current test/fuzz suite already goes through it as a two-argument
constructor pattern. **Recommendation:** mark `#[non_exhaustive]` in the next
minor release, since a future field (e.g. a human-readable announcement
date, or a source/provenance tag distinguishing IERS-table entries from
receiver-supplied ones) is plausible and currently blocked by the exposed
literal.

---

## API matrix

Organized by module, in the same order as `docs/ARCHITECTURE.md`'s layer
diagram.

### `scale` (layer 1)

| Item                                                               | Stability               | Notes                                                                                                                                                                                                                             |
| ------------------------------------------------------------------ | ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `TimeScale` (trait)                                                | 🟡 Stable — pending 1.0 | Sealed; see `docs/ARCHITECTURE.md#the-sealed-trait-pattern`. External impls are structurally impossible, so its "stability" is really about the associated-const contract, not extensibility.                                     |
| `Gps`, `Glonass`, `Galileo`, `Beidou`, `Tai`, `Utc` (marker types) | 🟢 Stable               | Zero-sized, `Copy`; adding a *new* marker type is additive (minor bump) and never affects these six.                                                                                                                              |
| `OffsetToTai`                                                      | 🟡 Stable — pending 1.0 | `#[non_exhaustive]`; see audit above.                                                                                                                                                                                             |
| `DisplayStyle`                                                     | 🟠 Unstable             | `#[non_exhaustive]`, but currently only consumed internally by `Display for Time<S>` — see [`#[doc(hidden)]` recommendation](#internal-items-that-should-be-dochidden) below; exposing it publicly may not have been intentional. |

### `epoch` (layer 1)

| Item                                                                                                        | Stability               | Notes                                                                                                                                                                            |
| ----------------------------------------------------------------------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CivilDate`                                                                                                 | 🟡 Stable — pending 1.0 | Plain struct, used both for scale-epoch definitions and (indirectly) `CivilDateTime`'s algorithms.                                                                               |
| `TAI_EPOCH`, `UNIX_EPOCH`, `UTC_CIVIL_EPOCH`, `GPS_EPOCH`, `GLONASS_EPOCH`, `GALILEO_EPOCH`, `BEIDOU_EPOCH` | 🟢 Stable               | Compile-time constants, `docs/INVARIANTS.md`-verified.                                                                                                                           |
| `UTC_EPOCH_UNIX_OFFSET_S`, `UTC_EPOCH_UNIX_OFFSET_NS`, `GPS_EPOCH_UNIX_S`                                   | 🟢 Stable               | Re-exported from `prelude`; see [re-export audit](#pub-use-re-export-audit).                                                                                                     |
| `LEAP_SECONDS_AT_*_EPOCH`, `DAYS_GPS_TO_*`, `NANOS_GPS_TO_*` constants                                      | 🟠 Unstable             | Public but not re-exported through `prelude`; likely intended as internal derivation constants rather than part of the primary API. Candidates for `#[doc(hidden)]` — see below. |

### `time` (layer 2)

| Item                                                                                                                                 | Stability               | Notes                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Time<S>`                                                                                                                            | 🟡 Stable — pending 1.0 | Core value type. 8-byte layout is an [invariant](INVARIANTS.md#i-18-8-byte-size), not just documentation.                                                                                                              |
| `Time::<S>::EPOCH`, `MIN`, `MAX`, `NANOS_PER_YEAR`                                                                                   | 🟢 Stable               | Associated consts.                                                                                                                                                                                                     |
| `from_nanos`, `from_seconds`, `checked_from_seconds`, `as_nanos`, `as_seconds`, `as_seconds_f64`                                     | 🟢 Stable               | Constructors/accessors present for all `S`.                                                                                                                                                                            |
| `to_tai`, `from_tai`, `try_convert`                                                                                                  | 🟡 Stable — pending 1.0 | The TAI-pivot mechanism itself ([I-10](INVARIANTS.md#i-10-tai-is-the-universal-pivot-for-fixed-offset-scales)); unlikely to change shape, but hasn't completed a full cycle.                                           |
| `checked_add`, `checked_sub_duration`, `saturating_add`, `saturating_sub_duration`, `try_add`, `try_sub_duration`, `checked_elapsed` | 🟢 Stable               | The four-form overflow policy is documented as an invariant ([I-9](INVARIANTS.md#i-9-overflow-policy-is-explicit-and-uniform)); changing any one form's *name* would be a minor bump but the *policy* itself is fixed. |
| `Add<Duration>`, `Sub<Duration>`, `Sub<Time<S>>`, `AddAssign`, `SubAssign`, `Ord`, `PartialOrd`, `Debug`, `Display`                  | 🟢 Stable               | Standard trait impls.                                                                                                                                                                                                  |
| `DurationParts`                                                                                                                      | 🟢 Stable               | See `#[non_exhaustive]` audit above for why it stays a plain struct.                                                                                                                                                   |
| `Time::<Glonass>::from_day_tod`, `day`, `tod_seconds`, `sub_second_nanos`, `day_of_week`, `is_weekend`                               | 🟢 Stable               | GLONASS-specific accessors.                                                                                                                                                                                            |
| `Time::<Gps>::from_week_tow`, `week`, `tow_seconds`, `sub_second_nanos`                                                              | 🟢 Stable               | Signature history (the `u16`→`u32` widening) is in the semver table above.                                                                                                                                             |
| `Time::<Gps>::from_unix_seconds`, `as_unix_seconds`, `to_utc`, `to_utc_with`                                                         | 🟡 Stable — pending 1.0 | Unix-interop methods added in `#TIME-21`.                                                                                                                                                                              |
| `Time::<Utc>::from_unix_seconds`, `from_unix_nanos`, `as_unix_seconds`, `as_unix_nanos`, `to_gps`, `to_gps_with`, `to_civil`         | 🟡 Stable — pending 1.0 | Same.                                                                                                                                                                                                                  |

### `duration` (used by layer 2, otherwise independent)

| Item                                                                                                                                    | Stability                    | Notes                                                                                                                                                                                                                                          |
| --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Duration`                                                                                                                              | 🟡 Stable — pending 1.0      | Signed interval type ([I-5](INVARIANTS.md#i-5-duration-is-signed)).                                                                                                                                                                            |
| `ZERO`, `MIN`, `MAX`, `ONE_NANOSECOND`                                                                                                  | 🟢 Stable                    | Associated consts.                                                                                                                                                                                                                             |
| `from_nanos`, `from_seconds`, `from_millis`                                                                                             | 🟢 Stable                    | Core constructors.                                                                                                                                                                                                                             |
| `from_minutes`, `from_hours`, `from_days`                                                                                               | 🟠 Unstable — **deprecated** | Flagged in `#TIME-27.1` for silent-overflow risk; `#[deprecated]` in favor of `checked_from_*`. Kept working (patch-safe per the semver table), scheduled for eventual removal (a **minor** bump) once `checked_from_*` has seen a full cycle. |
| `checked_from_seconds`, `checked_from_millis`, `checked_from_micros`, `checked_from_minutes`, `checked_from_hours`, `checked_from_days` | 🟡 Stable — pending 1.0      | Added in `#TIME-27.1`; the intended long-term replacement for the deprecated `from_*` family above.                                                                                                                                            |
| `checked_add`, `abs`, `neg` (`Neg`), `is_negative`, `as_seconds`, arithmetic operators                                                  | 🟢 Stable                    |                                                                                                                                                                                                                                                |

### `leap` (layer 3)

| Item                                        | Stability               | Notes                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------                                                       |
| `LeapSecondsProvider` (trait)               | 🟡 Stable — pending 1.0 | Public, **not** sealed — unlike `TimeScale`, third-party providers (e.g. a receiver-firmware-backed one) are an intended extension point. `tai_minus_utc_at`'s total `i32` return (never panics, never `None` — an empty table falls back to the builtin `19 s` offset) is itself an [invariant](INVARIANTS.md#i-16-when-can-gps--utc-produce-ambiguousleapsecond) fixed by the `#TIME-27.1` empty-table audit fix. |
| `LeapSeconds`                               | 🟡 Stable — pending 1.0 | `builtin()`, `from_table`, `try_from_slice`, `entries`, `last_update`, `current_tai_minus_utc`, `tai_minus_utc_at`.                                                                                                                                                                                                                                                                                                 |
| `RuntimeLeapSeconds`                        | 🟡 Stable — pending 1.0 | `new`, `from_builtin`, `from_slice` (→ `Result<Self, _>`), `try_extend`, `len`, `is_empty`, `entries`, `last_update`, `current_tai_minus_utc`. `try_extend`'s error-priority contract is fuzz-pinned (`fuzz_try_extend.rs`) — treat it as load-bearing, not incidental.                                                                                                                                             |
| `RUNTIME_CAPACITY`                          | 🟢 Stable               | `= 64`. Changing this value is a **minor** bump (changes `RuntimeLeapSeconds`'s size, an observable property per [I-18](INVARIANTS.md#i-18-8-byte-size)-adjacent reasoning even though `RuntimeLeapSeconds` itself isn't pinned to 8 bytes).                                                                                                                                                                        |
| `LeapEntry`                                 | 🟠 Unstable             | See `#[non_exhaustive]` recommendation above.                                                                                                                                                                                                                                                                                                                                                                       |
| `LeapExtendError`                           | 🟡 Stable — pending 1.0 | `#[non_exhaustive]`; variant set fuzz-pinned.                                                                                                                                                                                                                                                                                                                                                                       |
| `gps_to_utc`, `utc_to_gps` (free functions) | 🟢 Stable               | The two-pass algorithm's public entry points ([I-12](INVARIANTS.md#i-12-contextual-formula--gps--utc-and-the-two-pass-utc--gps-algorithm)); heavily fuzzed.                                                                                                                                                                                                                                                         |

### `convert` (layer 4)

| Item                                                        | Stability               | Notes                                                                                                                                                                                                         |
| ----------------------------------------------------------- | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `IntoScale<Target>` (trait)                                 | 🟢 Stable               | Fixed-offset conversion front door.                                                                                                                                                                           |
| `IntoScaleWith<Target>` (trait)                             | 🟢 Stable               | Contextual conversion front door; `into_scale_with` / `into_scale_with_checked`.                                                                                                                              |
| `ConvertResult<T>`                                          | 🟡 Stable — pending 1.0 | `is_exact`, `into_inner`, `Exact`/`AmbiguousLeapSecond`; `#[non_exhaustive]`.                                                                                                                                 |
| Per-pair `impl IntoScale<…>`/`impl IntoScaleWith<…>` blocks | 🟢 Stable               | Not separately nameable; their *existence* for a given pair is documented by `matrix.rs`'s `ConversionMatrix`, which is the supported way to query "does a conversion exist" rather than trait-bound probing. |

### `matrix` (layer 5)

| Item               | Stability               | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------ | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ScaleId`          | 🟠 Unstable             | `#[non_exhaustive]` enum listing the six scales for runtime introspection; adding a scale is already a patch-safe variant addition, matching how the rest of the `#TIME-31`/`#TIME-32`-style "add a new scale" checklist (`docs/ARCHITECTURE.md#extending-adding-a-new-time-scale`) is patch-safe.                                                                                                                                                                                                      |
| `ConversionKind`   | 🟡 Stable — pending 1.0 | `#[non_exhaustive]`; see audit above.                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `ConversionMatrix` | 🟠 Unstable             | Runtime introspection API (`path_count`, per-pair `conversion_kind` lookup); newest layer, least exercised by external callers so far.                                                                                                                                                                                                                                                                                                                                                                  |

### `civil`

| Item                                                                     | Stability               | Notes                                                                                                                                                                                                                                     |
| ------------------------------------------------------------------------ | ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------                                                                                            |
| `CivilDateTime`                                                          | 🟢 Stable               | See `#[non_exhaustive]` audit above for why it stays a plain struct; API unchanged since `0.5.3` (five release cycles).                                                                                                                   |
| `from_utc_nanos`, `to_utc_nanos`, `to_utc`, `is_whole_second`, `Display` | 🟢 Stable               | Algorithm is Howard Hinnant's public-domain `civil_from_days`; pinned by `src/civil.rs` unit tests and `tests/time_integration_test.rs`'s civil-date cross-checks. No dedicated fuzz target (`fuzz_civil` is not one of the six targets). |
| `Time::<Utc>::to_civil`                                                  | 🟢 Stable               | The only cross-module entry point; documented as infallible (see [the civil section of `docs/ARCHITECTURE.md`](ARCHITECTURE.md#civil-date-time-iso-8601)).                                                                                |

### `error`

| Item            | Stability | Notes                                                                                                                                                                                      |
| --------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `GnssTimeError` | 🟢 Stable | `#[non_exhaustive]`; `Overflow`/`InvalidInput`/`LeapSecondsRequired`/`OutOfRange`. `impl std::error::Error` gated on `feature = "std"`; `impl defmt::Format` gated on `feature = "defmt"`. |

### `serde_impls` (feature = `serde`)

| Item                                                                      | Stability                           | Notes                                                                                                                                                                                                                                              |
| ------------------------------------------------------------------------- | ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `impl Serialize`/`Deserialize` for `Time<S>`, `Duration`, `DurationParts` | 🟡 Stable — pending 1.0             | Wire format documented in `docs/ARCHITECTURE.md#serde-support-feature--serde` and `docs/EMBEDDED.md`; format is itself the contract, not the module's item list.                                                                                   |
| `pub mod serde_impls` (the module path itself)                            | 🔴 Internal — **should be private** | See [`#[doc(hidden)]` recommendation](#internal-items-that-should-be-dochidden) below. There is nothing in the module meant to be named by a caller — `impl` blocks are discovered automatically by the trait system, not through the module path. |

### `prelude`

| Item                                      | Stability | Notes                                                                                                                                                                                                           |
| ----------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `gnss_time::prelude::*` (the glob itself) | 🟢 Stable | The set of names it exports may **grow** (patch-safe, purely additive) but existing names will not be removed without a minor bump. See [re-export audit](#pub-use-re-export-audit) for the current exact list. |

---

## `pub use` re-export audit

`lib.rs` currently does:

```rust
pub use civil::CivilDateTime;
pub use convert::*;
pub use duration::*;
pub use epoch::*;
pub use error::*;
pub use leap::*;
pub use matrix::*;
pub use scale::*;
pub use time::*;
```

Each glob was checked against the item it actually re-exports at the crate
root (i.e. what becomes reachable as `gnss_time::Foo` instead of only
`gnss_time::module::Foo`):

| `pub use`              | What it surfaces at the crate root                                                                                                         | Verdict                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------                                                                                             |
| `civil::CivilDateTime` | one named type                                                                                                                             | ✅ Correct — deliberate single-item re-export, matches how `Time`/`Duration` are surfaced.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `convert::*`           | `ConvertResult`, `IntoScale`, `IntoScaleWith`                                                                                              | ✅ Correct — the whole point of layer 4 is to be the primary caller-facing API.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `duration::*`          | `Duration`, `DurationParts`                                                                                                                | ✅ Correct.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `epoch::*`             | `CivilDate`, all epoch constants (`TAI_EPOCH`, …, `UTC_EPOCH_UNIX_OFFSET_S`, …, `LEAP_SECONDS_AT_*`, `DAYS_GPS_TO_*`, `NANOS_GPS_TO_*`)    | ⚠️ **Over-broad.** This glob surfaces roughly 20 items at the crate root, most of which (`LEAP_SECONDS_AT_GLONASS_EPOCH`, `DAYS_GPS_TO_BEIDOU`, `NANOS_GPS_TO_GALILEO_EPOCH`, …) are derivation constants used internally by `epoch.rs`'s own `const` assertions and by nothing else in the crate. They are not referenced from `prelude.rs`, and their *values* already carry contract weight — each is pinned by `epoch.rs`'s own `const` assertions and `docs/INVARIANTS.md`'s worked examples — so re-exporting them adds public surface without adding flexibility. See recommendation below. |
| `error::*`             | `GnssTimeError`                                                                                                                            | ✅ Correct.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `leap::*`              | `LeapSecondsProvider`, `LeapSeconds`, `RuntimeLeapSeconds`, `LeapEntry`, `LeapExtendError`, `RUNTIME_CAPACITY`, `gps_to_utc`, `utc_to_gps` | ✅ Correct — all eight are referenced from `prelude.rs`, confirming intentional public status.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `matrix::*`            | `ConversionMatrix`, `ScaleId`, `ConversionKind`                                                                                            | ✅ Correct — `ScaleId` and `ConversionKind` are both `#[non_exhaustive]`, keeping future scale additions patch-safe.                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `scale::*`             | `TimeScale`, `Gps`, `Glonass`, `Galileo`, `Beidou`, `Tai`, `Utc`, `OffsetToTai`, `DisplayStyle`                                            | ⚠️ **`DisplayStyle` flagged.** The *type* must stay public — it is the type of the public associated const `TimeScale::DISPLAY_STYLE` — but it exists solely to let `Display for Time<S>` pick a format and is never taken or returned by a public function, so a caller has no reason to *name* it directly; `#[doc(hidden)]` (see below) hides it without breaking the associated-const contract.                                                                                                                                                                                                |
| `time::*`              | `Time`                                                                                                                                     | ✅ Correct.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

**Net verdict:** two gratuitous re-exports found — `epoch`'s internal
derivation constants, and `scale::DisplayStyle`. Neither is a *bug* (nothing
is broken), but both widen the public API surface with items that were
never meant to be depended on, which is exactly the gap `#[doc(hidden)]`
exists to close without changing behavior.

---

## Internal items that should be `#[doc(hidden)]`

`#[doc(hidden)]` keeps an item technically public (so existing code that
happens to reference it doesn't break) while removing it from generated
docs and signaling "do not use this" to anyone browsing the API. It is the
correct tool here because none of the items below can be made *actually*
private without breaking the glob re-export mechanism they're reached
through (`pub use epoch::*`, `pub use scale::*`) — the goal is to stop
advertising them, not to change what compiles.

| Item                                                                                                                          | Location   | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LEAP_SECONDS_AT_GPS_EPOCH`, `LEAP_SECONDS_AT_GLONASS_EPOCH`, `LEAP_SECONDS_AT_GALILEO_EPOCH`, `LEAP_SECONDS_AT_BEIDOU_EPOCH` | `epoch.rs` | Used only by `epoch.rs`'s own `const` assertions and `docs/INVARIANTS.md`'s worked examples. Not referenced by `prelude.rs`.                                                                                                                                                                                                                                                                                                    |
| `DAYS_GPS_TO_GALILEO`, `DAYS_GPS_TO_BEIDOU`, `DAYS_GPS_TO_GLONASS`, `DAYS_UNIX_TO_GPS`                                        | `epoch.rs` | Same — internal derivation only.                                                                                                                                                                                                                                                                                                                                                                                                |
| `NANOS_GPS_TO_GALILEO_EPOCH`, `NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR`                                                            | `epoch.rs` | Same.                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `DisplayStyle` (the type itself, and its variants)                                                                            | `scale.rs` | The type cannot be made private — it is the type of the public associated const `TimeScale::DISPLAY_STYLE` — but it exists solely for `Display for Time<S>`'s internal `match`, and no public function takes or returns a `DisplayStyle`; `#[doc(hidden)]` keeps `S::DISPLAY_STYLE` usable while signaling "do not name the type directly".                                                                                     |
| `pub mod serde_impls`                                                                                                         | `lib.rs`   | The module contains only trait `impl` blocks (auto-discovered by the compiler, never referenced by path) plus their private helper types (`TimeVisitor`, `DurationVisitor`, field-key enums, etc. — see `docs/ARCHITECTURE.md#serde-support-feature--serde`). Nothing in it needs a public path; `#[doc(hidden)] pub mod serde_impls;` keeps the impls active while removing an empty, confusing entry from `cargo doc` output. |
| `tables` module                                                                                                               | `lib.rs`   | Already `mod tables;` (private, not `pub mod`) — correct as-is; listed here only to confirm the audit checked it.                                                                                                                                                                                                                                                                                                               |

**What should *not* be hidden**, for contrast: `LeapEntry`, despite being
flagged for `#[non_exhaustive]` above, is genuinely public API (constructing
a custom leap-second table requires it) and must stay fully documented.

---

## Doc-test coverage

Every `rust` fenced code block in a public-item doc comment was reviewed for
whether it compiles as a normal doctest (i.e. is not marked `ignore`,
`no_run` where `run` would in fact work, or `compile_fail` where the intent
is actually runnable code).

**Audit result:** every `rust` code block currently in the crate's doc
comments is a plain, unannotated fence — meaning `cargo test --doc` already
compiles and runs all of them with no `ignore` escape hatches anywhere in
the crate. This includes the module-level examples in `lib.rs`,
`prelude.rs`, `civil.rs`, and every method-level example across `time.rs`,
`duration.rs`, `leap.rs`, `convert.rs`, and `epoch.rs`.

The one legitimate use of a non-default annotation is
`examples/no_domain_mixing.rs`, which is a *standalone example file*
demonstrating a **compile error** (mixing `Time<Gps>` and `Time<Glonass>`
arithmetic) — this is not a doctest and is not built by `cargo test --doc`
at all; it exists purely as a documented, human-readable illustration
referenced from `docs/INVARIANTS.md` ([I-1](INVARIANTS.md#i-1-domain-isolation))
and is correctly excluded from the crate's `[[example]]` build list for that
reason (it would fail `cargo build --examples` otherwise, which is the
point).

**No action needed** beyond continuous enforcement — see below.

---

## CI enforcement

`cargo test --doc` was not previously a named, separate CI step (it runs
implicitly as part of `cargo test`, but a failure there is easy to miss
among hundreds of unit/integration test names). Add an explicit step so a
broken doctest is unambiguous in CI output:

```yaml
# .github/workflows/ci.yml — add alongside the existing test job
  doctest:
    name: doc-tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: cargo test --doc (default features)
        run: cargo test --doc
      - name: cargo test --doc (all features)
        run: cargo test --doc --all-features
```

Running it once with default features and once with `--all-features`
matters specifically because of the `# #[cfg(feature = "serde")] { … }`
doc examples in `lib.rs` and `civil.rs`'s serde-gated snippets — a doctest
inside a `#[cfg(feature = "...")]` block is silently skipped (not failed)
when that feature is off, so the `--all-features` run is the only one that
actually compiles and executes those examples.

`justfile` gets a matching recipe for local use:

```just
# Run all doc-tests (default + all-features)
doctest:
    cargo test --doc
    cargo test --doc --all-features
```

and the existing `ci` recipe (`lint test-all check-embedded doc-check`)
should call it: `ci: lint test-all doctest check-embedded doc-check`.
