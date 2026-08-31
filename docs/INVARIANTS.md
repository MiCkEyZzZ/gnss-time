# Invariants and safety guarantees

This document lists the invariants that `gnss-time` upholds, along with the
mechanisms that enforce them.

## Type-level invariants

### I-1: Domain isolation

`Time<A>` and `Time<B>` values (where `A ≠ B`) cannot be mixed in arithmetic
expressions.

**Enforcement:** the Rust type system. The `Sub<Time<S>>` and `Add<Duration>`
impls exist only for `Time<S>` with the same `S`. Any attempt to subtract a
GLONASS timestamp from a GPS timestamp results in a compile error.

### I-2: No implicit conversions

There are no `From` / `Into` implementations between different scales. Every
conversion is done explicitly through a call to `into_scale()` or
`into_scale_with(ls)`.

**Enforcement:** the absence of blanket implementations. All the
`IntoScale` / `IntoScaleWith` implementations are written by hand and verified.

### I-3: Sealed time scales

External code cannot implement `TimeScale`. The set of valid scales is:
`{Gps, Glonass, Galileo, Beidou, Tai, Utc}`.

**Enforcement:** the `private::Sealed` supertrait pattern. The `Sealed` trait
lives in a private module and has no public path.

## Arithmetic invariants

### I-4: No silent overflow

All `+` and `-` operators for `Time<S>` and `Duration` panic on overflow. For
code where panicking is unacceptable, checked/saturating/fallible variants are
provided.

**Enforcement:** wrapping arithmetic is not used. The
`#[deny(arithmetic_overflow)]` lint (via `-D warnings` in CI) catches any
accidental overflow at compile time. `#[allow(arithmetic_overflow)]` is banned
in CI.

### I-5: `u64::MAX` is the hard limit

`Time::<S>::MAX.as_nanos() == u64::MAX`. No operation can create a value
larger than this; instead it either panics, returns `None`, saturates, or
returns `Err`.

**Enforcement:** all arithmetic is done in `i128` with a range check before
casting back to `u64`.

### I-6: `Duration` is signed

`Duration` uses `i64` nanoseconds. Subtracting a later time from an earlier
one yields a negative `Duration`. This makes it natural to work in both
directions.

## Conversion invariants

### I-7: TAI is the universal pivot

```text
T_tai = T_self + S::OFFSET_TO_TAI
```

This equation holds for all scales with a fixed offset (`Gps`, `Galileo`,
`Beidou`, `Tai`). All pairwise conversions are derived from this formula.
There is no "magic" for specific pairs of scales.

**Enforcement:** `try_convert<T>` calls `to_tai()`, then `T::from_tai()`. No
conversion bypasses TAI.

### I-8: GPS–Galileo identity

GPS and Galileo have the same offset
`OFFSET_TO_TAI = 19_000_000_000 ns`. Therefore,
`T_gps.as_nanos() == T_gal.as_nanos()` for the same physical moment.

**Test:** `test_gps_galileo_identity_via_tai` in `src/time.rs`.

### I-9: Fixed GPS–BeiDou offset

`BDT = GPS − 14s` always. This follows from `GPS+19 = BDT+33 = TAI`.

**Test:** `test_gps_to_beidou_subtracts_14_seconds` in `src/time.rs`.

### I-10: GLONASS–UTC epoch offset

The GLONASS epoch = 1995-12-31 21:00:00 UTC = 757 371 600 seconds from the UTC
epoch (1972-01-01). This is a compile-time constant, verified as follows:

```rust
const _VERIFY_GLONASS_OFFSET: () = {
    assert!(GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000 == 757_371_600);
};
```

### I-11: Correctness of the two-pass UTC → GPS algorithm

The two-pass `utc_to_gps` algorithm is correct at all 18 leap-second
boundaries of the GPS era.
**Tests:** `prop_all_18_leap_second_transitions_correct` in
`tests/prop_tests.rs` and the individual transition tests in `src/leap.rs`.

### I-12: Roundtrip accuracy outside the ambiguity window

For any `t: Time<Gps>` that does **not** fall into the 1-second leap-second
ambiguity window:
`gps_to_utc(utc_to_gps(t, ls), ls) == t`.

**Test:** `prop_gps_utc_gps_roundtrip_for_all_samples` in
`tests/prop_tests.rs` (256 points; the ambiguity window is skipped via
`AmbiguousLeapSecond`).

## Memory invariants

### I-13: No heap allocation

`Time<S>` and `Duration` are `Copy` types without a `Drop` implementation.
`LeapSeconds::builtin()` returns a `&'static LeapSeconds` pointing to a static
array. The `alloc` crate is not used anywhere.

**Enforcement:** `#![no_std]` in `lib.rs` without `extern crate alloc`.
The `all_conversions_are_stack_only` test in `tests/no_std_compat.rs`.

### I-14: 8-byte size

`size_of::<Time<S>>() == 8` for all `S: TimeScale`.

**Enforcement:** the unit test `test_size_equals_u64` runs in CI through the
`type-sizes` job in `.github/workflows/embedded.yml`.

## Safety invariants

### I-15: No unsafe code

`#![forbid(unsafe_code)]` in `lib.rs`. Any attempt to add unsafe code is a
compile error, not a warning.

**CI check:** `grep -n "forbid(unsafe_code)" src/lib.rs` in the `lint` job.

### I-16: No missing documentation

`#![deny(missing_docs)]` in `lib.rs`. Every public item is required to have
documentation.

**CI check:** `cargo clippy -- -D warnings` in the `lint` job.
