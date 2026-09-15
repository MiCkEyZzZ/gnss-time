# Invariants and safety guarantees

This document lists the invariants that `gnss-time` upholds, the formulas
behind them, and the mechanisms — types, `const` assertions, unit tests,
property tests, and fuzz targets — that enforce each one.

Every invariant below has a **Test** line. If you break an invariant, one of
those tests should fail; if it doesn't, the test is the bug.

## Table of contents

- [Type-level invariants](#type-level-invariants)
- [Representation invariants](#representation-invariants) — why `u64`, not `i64`/`f64`
- [Arithmetic invariants](#arithmetic-invariants) — overflow behavior
- [Conversion formulas](#conversion-formulas)
- [Roundtrip guarantees](#roundtrip-guarantees)
- [The leap-second ambiguity window](#the-leap-second-ambiguity-window)
- [Memory invariants](#memory-invariants)
- [Safety invariants](#safety-invariants)
- [Invariant ↔ test cross-reference](#invariant--test-cross-reference)

---

## Type-level invariants

### I-1: Domain isolation

`Time<A>` and `Time<B>` values (where `A ≠ B`) cannot be mixed in arithmetic
expressions.

**Enforcement:** the Rust type system. The `Sub<Time<S>>` and `Add<Duration>`
impls exist only for `Time<S>` with the same `S` (see `src/time.rs`:
`impl Add<Duration> for Time<S>`, `impl Sub<Duration> for Time<S>`,
`impl Sub<Time<S>> for Time<S>`). Any attempt to subtract a GLONASS timestamp
from a GPS timestamp results in a compile error.

**Test:** `examples/no_domain_mixing.rs` (a `// does not compile` example);
enforced structurally — there is no runtime test to write, because the
violation cannot reach runtime.

### I-2: No implicit conversions

There are no `From` / `Into` implementations between different scales. Every
conversion is done explicitly through a call to `into_scale()` or
`into_scale_with(ls)`. See `docs/ARCHITECTURE.md#limitations` for the
rationale (fallibility and the required leap-second context don't fit
`From`'s infallible contract).

**Enforcement:** the absence of blanket implementations. All the
`IntoScale` / `IntoScaleWith` implementations are written by hand, one per
ordered pair, and their completeness is checked by the exhaustive pairwise
tests in `matrix.rs`.

### I-3: Sealed time scales

External code cannot implement `TimeScale`. The set of valid scales is:
`{Gps, Glonass, Galileo, Beidou, Tai, Utc}`.

**Enforcement:** the `private::Sealed` supertrait pattern (`src/scale.rs:39`).
The `Sealed` trait lives in a private module and has no public path.

**Test:** `test_scale_types_are_copy` (`src/scale.rs:304`) and friends in
`src/scale.rs` confirm all six marker types implement the trait; sealing
itself is a compile-time property with no positive runtime test (a would-be
violation is a compile error in downstream code, not a test case in this
crate).

---

## Representation invariants

### I-4: Nanoseconds are stored as `u64`, not `i64` or `f64`

`Time<S>` stores `nanos: u64` — an **unsigned** count of nanoseconds since
`S`'s epoch. This is a deliberate choice with three parts:

**Why not signed (`i64`)?** A `Time<S>` is a *point in time*, not an
*interval* — negative values would mean "before the scale's epoch," which no
supported scale needs to represent (every scale's usable range starts at or
after its own epoch by definition). Making the type unsigned turns "time
before this scale existed" into a type-level impossibility rather than a
runtime check: `Time::<S>::EPOCH` (0 ns, `src/time.rs:128`) is the smallest
representable instant, full stop. This is also why `checked_sub_duration` and
the `Sub` operator can fail on *positive* durations near the epoch —
subtracting past zero is `Overflow`/panic, not a negative result.

Contrast with `Duration`, which *is* signed (`i64`) precisely because it
represents a difference between two instants and must be able to express
"earlier than" — see [I-5](#i-5-duration-is-signed).

**Why not floating point (`f64`)?** `f64` has 52 bits of mantissa, enough for
exact integers only up to 2^53 ≈ 9.007 × 10^15. A nanosecond-resolution
timestamp reaches that magnitude in about 104 days
(2^53 ns ≈ 104.25 days) — far short of any GNSS scale's useful lifetime.
Past that point, `f64` silently rounds to the nearest representable value:
two distinct nanosecond instants could compare equal, and arithmetic would be
non-associative in ways that break the roundtrip guarantees below. Integer
nanoseconds have none of these failure modes: every representable `u64` value
is exact, and comparisons/arithmetic are exact until they overflow, at which
point the crate's overflow policy (see [I-9](#i-9-overflow-policy-is-explicit-and-uniform))
takes over instead of silently losing precision.

**Why 64 bits specifically?** `u64::MAX` nanoseconds ≈ 584.5 years — enough
headroom past every scale's epoch that overflow is a genuine edge case
(reachable only near `Time::<S>::MAX`, exercised deliberately by the
`fuzz_gps_utc`/`fuzz_week_tow` boundary-mode fuzz targets) rather than an
everyday concern. A `u32` count (≈ 4.29 seconds of nanosecond resolution)
would be useless; `u128` would double the type's size for no benefit within
any scale's practical lifetime.

**Test:** `test_size_equals_u64` (`src/time.rs:1131`, confirms the 8-byte,
`u64`-identical layout); the `f64`-precision-loss argument above is
structural, not runtime-tested (there is no `f64`-backed alternative type in
the crate to compare against).

### I-5: `Duration` is signed

`Duration` uses `i64` nanoseconds (`src/duration.rs:94`, `#[repr(transparent)]`).
Subtracting a later time from an earlier one yields a negative `Duration`, and
it can be added back to *either* operand to recover the other. This is the
interval counterpart to [I-4](#i-4-nanoseconds-are-stored-as-u64-not-i64-or-f64):
a `Time<S>` is a point (`u64`, unsigned by construction), a `Duration` is a
displacement between two points (`i64`, signed by necessity).

**Test:** `test_sub_times_negative` (`src/time.rs:1280`),
`test_negative` (`src/duration.rs:592`).

---

## Arithmetic invariants

### I-6: No silent overflow

The `+` and `-` operators for `Time<S>` and `Duration` **panic** on
overflow. This is deliberate, not an oversight — see
[I-9](#i-9-overflow-policy-is-explicit-and-uniform) for the full policy and
why panicking is the *default* rather than the *only* option.

**Enforcement:** wrapping arithmetic is never used. The `-D warnings` flags in
CI (`RUSTFLAGS` in `.github/workflows/ci.yml`) escalate the
deny-by-default `arithmetic_overflow` lint, which fires on compile-time
constant overflow, so any such overflow fails the main build; the panicking
operator implementations catch every *runtime* overflow.
`#[allow(arithmetic_overflow)]` is banned anywhere in the crate.

**Test:** `test_add_operator_panics_at_max` (`src/time.rs:1671`),
`test_sub_operator_panics_at_epoch` (`src/time.rs:1677`);
`#[should_panic]` tests in `src/duration.rs`.

### I-7: `u64::MAX` is the hard upper limit; `EPOCH` is the hard lower limit

`Time::<S>::MAX.as_nanos() == u64::MAX` and
`Time::<S>::MIN == Time::<S>::EPOCH == Time::from_nanos(0)`
(`src/time.rs:134`). No operation can create a `Time<S>` value outside
`[EPOCH, MAX]`; every arithmetic path either panics, returns `None`,
saturates to one of these two bounds, or returns `Err(Overflow)` — see
[I-9](#i-9-overflow-policy-is-explicit-and-uniform).

**Enforcement:** all arithmetic is performed in `i128` with an explicit range
check against `[0, u64::MAX]` before casting back to `u64`
(`to_tai`, `from_tai`, `try_convert`, `checked_add`,
`checked_sub_duration`, `checked_elapsed` in `src/time.rs`); the cast itself
cannot silently wrap because the range check happens first.

**Test:** `test_max_is_u64_max` (`src/time.rs:1548`),
`test_checked_add_at_max_overflows` (`src/time.rs:1583`),
`test_checked_sub_at_epoch_underflows` (`src/time.rs:1605`);
`fuzz_week_tow`/`fuzz_day_tod` RAW mode exercises the entire input domain of
the constructors that produce `Time<S>` values, so any path that could escape
`[EPOCH, MAX]` would surface as an assertion failure there.

### I-8: `checked_elapsed` fits the difference into `i64`

`Time<S>::checked_elapsed(earlier)` (`src/time.rs:374`) computes
`self − earlier` as a `Duration` (`i64` nanoseconds) and returns `None` if
the true difference does not fit in `i64` — which is possible because
`Time<S>` spans the full `u64` range (≈ 584.5 years) while `Duration` spans
only `i64` (≈ ±292 years): the gap between `Time::<S>::MIN` and
`Time::<S>::MAX` is exactly twice what a single `Duration` can express.

**Test:** `test_checked_elapsed_overflows_when_gap_exceeds_i64` (`src/time.rs:1689`),
`test_checked_elapsed_within_i64_range_works` (`src/time.rs:1698`).

### I-9: Overflow policy is explicit and uniform

Every fallible arithmetic operation on `Time<S>` and `Duration` is offered in
(up to) four forms, and the crate never mixes them silently — the *name* of
the method tells you the failure mode:

| Suffix / form           | On overflow…                                           | Use when                                                                                         |
| ----------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `+` / `-` (operators)   | **panics**                                             | overflow is a programmer error you want to catch immediately (default; matches `std` convention) |
| `checked_*`             | returns **`None`**                                     | you want to branch on failure without allocating an error type                                   |
| `saturating_*`          | **clamps** to `EPOCH`/`MAX` (or `Duration::MIN`/`MAX`) | embedded/control loops where a panic would be worse than a clamped value                         |
| `try_*`                 | returns **`Err(GnssTimeError::Overflow)`**             | you want the failure integrated into a `Result`-based call chain (`?`)                           |

No operation silently wraps (two's-complement wraparound) or silently loses
precision — those are the two failure modes explicitly ruled out by
[I-6](#i-6-no-silent-overflow) and [I-4](#i-4-nanoseconds-are-stored-as-u64-not-i64-or-f64)
respectively. The four forms above are the *complete* set of legal behaviors;
if you find a fifth (e.g. a method that returns a wrong-but-valid value
instead of one of these four), that is a bug.

**Test:** `test_time_max_behavior` (`src/time.rs:1533`) exercises all three
non-panicking forms (`checked_add`, `saturating_add`, `try_add`) against the
same overflowing input and asserts each returns the documented outcome;
paired with the `#[should_panic]` tests in [I-6](#i-6-no-silent-overflow) for
the operator form.

---

## Conversion formulas

### I-10: TAI is the universal pivot for fixed-offset scales

```text
T_tai = T_self + S::OFFSET_TO_TAI
```

This equation holds for every scale with `OffsetToTai::Fixed` (`Gps`,
`Galileo`, `Beidou`, `Tai` itself with offset 0). All pairwise conversions
between such scales are derived from this one formula composed twice
(`to_tai()` then `from_tai()`, i.e. `try_convert::<T>()` — `src/time.rs:275`)
— there is no per-pair special case.

**Enforcement:** `try_convert<T>` calls `to_tai()`, then `T::from_tai()`. No
fixed-offset conversion bypasses TAI.

**Test:** `test_into_scale_gps_tai_matches_to_tai` (`src/convert.rs:916`);
`test_roundtrip_via_tai` (`src/time.rs:1339`).

### I-11: Fixed-offset formulas, scale by scale

| Conversion          | Formula                                                                                   | Fixed offset used                                                         |
| ------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| GPS → TAI           | `T_tai = T_gps + 19 s`                                                                    | `Gps::OFFSET_TO_TAI`                                                      |
| Galileo → TAI       | `T_tai = T_gal + 19 s`                                                                    | `Galileo::OFFSET_TO_TAI`                                                  |
| BeiDou → TAI        | `T_tai = T_bdt + 33 s`                                                                    | `Beidou::OFFSET_TO_TAI`                                                   |
| GPS ↔ Galileo       | `T_gal.as_nanos() = T_gps.as_nanos()` (identity — see [I-14](#i-14-gps-galileo-identity)) | both `+19 s`                                                              |
| GPS ↔ BeiDou        | `T_bdt = T_gps − 14 s`                                                                    | `19 s − 33 s = −14 s`                                                     |
| Galileo ↔ BeiDou    | `T_bdt = T_gal − 14 s`                                                                    | same as GPS↔BeiDou (via TAI)                                              |
| GLONASS ↔ UTC       | `T_utc = T_glo + 757 371 600 s`                                                           | epoch shift, **not** via TAI — see [I-13](#i-13-glonass-utc-epoch-offset) |

Every row except GLONASS↔UTC is a direct consequence of
[I-10](#i-10-tai-is-the-universal-pivot-for-fixed-offset-scales); GLONASS↔UTC
is `IntoScale` (fixed, no leap-second context needed) but does not go through
TAI, because GLONASS's `OffsetToTai` is `Contextual`, not `Fixed` — the
GLONASS↔UTC relationship is fixed *relative to UTC*, not relative to TAI.

**Test:** one test per row exists in `src/convert.rs` (`test_gps_to_tai_adds_19_seconds`,
`src/convert.rs:632`; `test_gps_to_beidou_subtracts_14_seconds`,
`src/convert.rs:707`; `test_glonass_epoch_to_utc_nanos`, `src/convert.rs:749`;
...) and `src/time.rs` (`test_roundtrip_via_tai`, `src/time.rs:1339`) — see
the [cross-reference table](#invariant--test-cross-reference) for the full
list.

### I-12: Contextual formula — GPS → UTC, and the two-pass UTC → GPS algorithm

```text
UTC_ns_from_1972 = GPS_ns_from_1980 − (TAI_minus_UTC(t) − 19) × 1e9
                    + UTC_TO_GPS_EPOCH_NS
```

where `TAI_minus_UTC(t)` is looked up from the `LeapSecondsProvider` at the
TAI instant corresponding to the input GPS time, and
`UTC_TO_GPS_EPOCH_NS = 252 892 800 × 1e9` (`src/leap.rs:82`) is the constant
offset between the UTC epoch (1972-01-01) and the GPS epoch (1980-01-06).
The `−19` subtracts out the offset already baked into GPS↔TAI, leaving only
the leap seconds accumulated *since* the GPS epoch. The same constant appears
in the formula's code comments and is const-asserted to be exactly
252 892 800 s (2927 days).

The reverse direction, `utc_to_gps`, uses a **two-pass** algorithm
(`src/leap.rs:952`): the first pass computes TAI approximately assuming
`GPS − UTC = 0`, and the second pass refines the result using the leap-second
count found on the first pass. This is what makes the conversion correct at
every one of the 18 leap-second boundaries of the GPS era.

**Test:** `test_gps_leads_utc_by_18s_at_2017_01_01` (`src/convert.rs:828`)
and `test_gps_leads_utc_by_13s_at_1999_01_01` (`src/convert.rs:843`) pin the
formula at two transition dates; `tests/roundtrip_test.rs::test_all_gps_era_leap_second_transitions`
covers the same formula at **all 18** transitions, as does the
`BOUNDARY_SECONDS` list in `tests/prop_tests.rs:282`;
`fuzz_gps_utc.rs` BOUNDARY mode exercises this formula at all 18 historical
transitions plus jitter.

### I-13: GLONASS–UTC epoch offset

The GLONASS epoch = 1995-12-31 21:00:00 UTC = 757 371 600 seconds from the
UTC epoch (1972-01-01). This is a compile-time constant, verified as follows
(`src/leap.rs:68`):

```rust
const _VERIFY_GLONASS_OFFSET: () = {
    assert!(GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000 == 757_371_600);
};
```

**Test:** `test_glonass_epoch_offset_is_757371600_seconds` (`src/leap.rs:1072`);`
`test_glonass_epoch_offset_from_utc_epoch_is_correct` (`src/leap.rs:1087`)
cross-checks the same constant against independent `CivilDate` arithmetic.

### I-14: GPS–Galileo identity

GPS and Galileo have the *same* fixed offset,
`OFFSET_TO_TAI = 19 000 000 000 ns` (`src/scale.rs:169`/`:183`). Therefore, for
the same physical moment, `T_gps.as_nanos() == T_gal.as_nanos()` — the
conversion is a type change with zero arithmetic (`ConversionKind::Identity`
in `src/matrix.rs:25`).

**Test:** `test_gps_galileo_identity_via_tai` (`src/time.rs:1347`);
`test_gps_galileo_is_identity` (`src/matrix.rs:318`).

---

## Roundtrip guarantees

### I-15: When does `A → B → A == A` hold?

| Conversion class                                                                                                  | Roundtrip guarantee                                                                                                                                                                    |
| ----------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Identity` (GPS ↔ Galileo)                                                                                        | **Always exact.** No arithmetic occurs; the nanosecond count is unchanged.                                                                                                             |
| `Fixed` (GPS ↔ TAI/BeiDou, Galileo ↔ BeiDou/TAI)                                                                  | **Always exact** for any value that does not overflow either direction. The `to_tai`/`from_tai` pair is a pure integer add/subtract with no rounding.                                  |
| `EpochShift` (GLONASS ↔ UTC)                                                                                      | **Always exact** for any value that does not overflow (in particular, UTC values before the GLONASS epoch cannot round-trip *into* GLONASS — they fail with `Overflow`, not silently). |
| `Contextual`, **outside** the leap-second window (GPS ↔ UTC, GPS ↔ GLONASS, and their Galileo/BeiDou equivalents) | **Exact.** `gps_to_utc(utc_to_gps(t, ls), ls) == t` for every `t` for which `into_scale_with_checked` reports `ConvertResult::Exact`.                                                  |
| `Contextual`, **inside** the leap-second window                                                                   | **Not guaranteed exact** — see [I-16](#i-16-the-leap-second-ambiguity-window). `into_scale_with_checked` reports `ConvertResult::AmbiguousLeapSecond` precisely so callers can detect this case instead of trusting a possibly-off-by-one-second result. |

The precise boundary for "outside the window" in the contextual row is
exactly the set of instants classified `Exact` by `into_scale_with_checked` —
this is *by definition*, not an approximation of it, since `ConvertResult`
exists specifically to make that boundary queryable rather than inferred.

**Test:** `test_gps_utc_gps_roundtrip_at_gps_epoch` (`src/leap.rs:1295`),
`test_gps_utc_gps_roundtrip_at_2020` (`src/leap.rs:1305`),
`test_gps_utc_roundtrip_exact_at_nanosecond_level` (`src/convert.rs:817`);
`prop_gps_utc_gps_roundtrip_exact` in `tests/prop_tests.rs:127` (256 sampled
points, ambiguity window excluded); `fuzz_gps_utc.rs` / `fuzz_utc_to_gps.rs`
invariant **I-12** in those harnesses' numbering (roundtrip accuracy — see
the harness doc comments) checks exactness on the `Exact` branch and a ≤1s
bound on the `AmbiguousLeapSecond` branch for the *entire* `u64` domain, not
just sampled points.

---

## The leap-second ambiguity window

### I-16: When can `GPS → UTC` produce `AmbiguousLeapSecond`?

A leap-second insertion adds one extra UTC second (23:59:60) that has no GPS
counterpart — GPS time never repeats or skips a second, by construction.
Concretely, around each of the 18 historical insertions, the mapping from
GPS nanoseconds to UTC nanoseconds is **not injective** for exactly the
one-second window immediately *before* the insertion: both the last regular
GPS second and the following leap GPS second are candidates for mapping to
the same UTC label, depending on which side of the insertion instant the
UTC value is interpreted as observing from.

`into_scale_with_checked` detects this by comparing `TAI − UTC` at the
queried instant against `TAI − UTC` one second earlier
(`src/convert.rs:530`):

```text
n_now    = tai_minus_utc_at(tai)
n_before = tai_minus_utc_at(tai − 1s)

n_now != n_before  ⇒  ConvertResult::AmbiguousLeapSecond
n_now == n_before  ⇒  ConvertResult::Exact
```

This means the ambiguous window is **exactly** one second wide, anchored at
each transition threshold present in the `LeapSecondsProvider` (all 18 in the
built-in table) — never wider, never narrower, and never present anywhere
else.

**What callers should do:** `gps_to_utc`/`utc_to_gps` (the free functions,
and `into_scale_with`) always return a single best-effort `Time<Target>`
even inside the window — they never refuse to answer. Use
`into_scale_with_checked` when the distinction matters (e.g. logging,
auditing, or anything where a 1-second discrepancy at a leap-second boundary
would be a correctness issue for the caller) and inspect the
`ConvertResult` variant.

**Test:** `test_gps_to_utc_detects_leap_second_ambiguity` (`src/convert.rs:936`),
`test_leap_second_transition_1999_gps_jumps_by_2s` (`src/leap.rs:1375`),
`test_leap_second_transition_2017_gps_jumps_by_2s` (`src/leap.rs:1400`);
`prop_ambiguous_only_near_boundaries` in `tests/prop_tests.rs:311` checks
this for all 18 transitions programmatically rather than the two hand-picked
ones above; `prop_gps_near_leap_converts_consistently`
(`tests/prop_tests.rs:336`) checks every transition; `fuzz_gps_utc.rs`/
`fuzz_utc_to_gps.rs` BOUNDARY mode is specifically constructed to land inside
every one of these windows on every fuzzing run (see those harnesses' doc
comments for why blind `u64` mutation cannot reliably reach a 2-second-wide
target in an ≈1.8×10^19-point space).

---

## Memory invariants

### I-17: No heap allocation

`Time<S>` and `Duration` are `Copy` types without a `Drop` implementation.
`LeapSeconds::builtin()` returns a `&'static LeapSeconds` pointing to a static
array (`src/leap.rs:50`). `RuntimeLeapSeconds` is a fixed-size stack/static
buffer (`[LeapEntry; RUNTIME_CAPACITY]`, `src/leap.rs:226`), not a `Vec`. The
`alloc` crate is not used anywhere in the crate's own code, with or without
the `serde` feature (see `docs/ARCHITECTURE.md#serde-support-feature--serde`).

**Enforcement:** `#![no_std]` in `src/lib.rs:55` without `extern crate alloc`.

**Test:** `test_no_heap_allocation_in_conversions` in
`tests/no_std_compact.rs:193`; the `no-std-transitive` CI job in
`.github/workflows/embedded.yml` greps the dependency tree for `std` across
every feature combination.

### I-18: 8-byte size

`size_of::<Time<S>>() == 8` for all `S: TimeScale`, and `size_of::<Duration>()
== 8`.

**Enforcement:** the layout is `{ nanos: u64, _scale: PhantomData<S> }` with
`PhantomData` contributing zero bytes; `Duration` is `#[repr(transparent)]`
over `i64` (`src/duration.rs:94`).

**Test:** `test_size_equals_u64` (`src/time.rs:1131`) runs in the standard
test suite and is re-verified for the embedded target in the `type-sizes` job
in `.github/workflows/embedded.yml`; the `firmware/` size-probe crate
additionally confirms the *compiled* representation matches (no hidden
padding introduced by a specific target ABI).

---

## Safety invariants

### I-19: No unsafe code

`#![forbid(unsafe_code)]` in `src/lib.rs:56`. Any attempt to add unsafe code
is a compile error, not a warning or a lint that can be silenced locally.

**Test:** `#![forbid(...)]` (vs. `#![deny(...)]`) cannot be overridden by an
inner `#[allow(unsafe_code)]`, so this is enforced by `rustc` itself on every
build, not by a CI grep.

### I-20: No missing documentation

`#![deny(missing_docs)]` in `src/lib.rs:57`. Every public item is required to
have documentation.

**Test:** `#![deny(missing_docs)]` is enforced by `rustc`/`cargo doc` on every
build (and verified by a grep in the CI `lint` job); `cargo clippy --all-targets -- -D warnings`
runs in the `clippy` CI job for all feature combinations.

---

## Invariant ↔ test cross-reference

Quick lookup from invariant to the tests that pin it, grouped by source.
This table is the inverse index of the per-invariant **Test** lines above —
use it when a test fails and you need to know *which* invariant it guards, or
when changing a formula and you need to know every place that depends on it.

| Invariant                        | Unit tests | Property / fuzz tests |
| -------------------------------- | ---------- | ----------------------- |
| I-1 Domain isolation             | `examples/no_domain_mixing.rs` (compile-fail) | — |
| I-2 No implicit conversions      | exhaustive `impl` review in `matrix.rs` tests | — |
| I-3 Sealed scales                | `src/scale.rs::test_scale_types_are_copy` | — |
| I-4 `u64` representation         | `src/time.rs::test_size_equals_u64` | — |
| I-5 `Duration` signed            | `src/duration.rs::test_negative`, `src/time.rs::test_sub_times_negative` | — |
| I-6 No silent overflow           | `src/time.rs::test_add_operator_panics_at_max`, `src/duration.rs` `#[should_panic]` tests | — |
| I-7 `[EPOCH, MAX]` bound         | `src/time.rs::test_max_is_u64_max`, `::test_checked_add_at_max_overflows`, `::test_checked_sub_at_epoch_underflows` | `fuzz/fuzz_targets/fuzz_week_tow.rs`, `fuzz_day_tod.rs` (RAW mode) |
| I-8 `checked_elapsed` fits `i64` | `src/time.rs::test_checked_elapsed_overflows_when_gap_exceeds_i64`, `::test_checked_elapsed_within_i64_range_works` | — |
| I-9 Uniform overflow policy      | `src/time.rs::test_time_max_behavior` | — |
| I-10 TAI pivot                   | `src/convert.rs::test_into_scale_gps_tai_matches_to_tai`, `src/time.rs::test_roundtrip_via_tai` | — |
| I-11 Per-scale formulas          | `src/convert.rs::test_gps_to_tai_adds_19_seconds`, `::test_gps_to_beidou_subtracts_14_seconds`, `::test_glonass_epoch_to_utc_nanos` | — |
| I-12 GPS→UTC formula + two-pass  | `src/convert.rs::test_gps_leads_utc_by_18s_at_2017_01_01`, `::test_gps_leads_utc_by_13s_at_1999_01_01`, `tests/roundtrip_test.rs::test_all_gps_era_leap_second_transitions` | `tests/prop_tests.rs::BOUNDARY_SECONDS`; `fuzz/fuzz_targets/fuzz_gps_utc.rs` BOUNDARY mode |
| I-13 GLONASS–UTC offset          | `src/leap.rs::test_glonass_epoch_offset_is_757371600_seconds`, `::test_glonass_epoch_offset_from_utc_epoch_is_correct` | — |
| I-14 GPS–Galileo identity        | `src/time.rs::test_gps_galileo_identity_via_tai`, `src/matrix.rs::test_gps_galileo_is_identity` | — |
| I-15 Roundtrip guarantees        | `src/leap.rs::test_gps_utc_gps_roundtrip_at_gps_epoch`, `::test_gps_utc_gps_roundtrip_at_2020`, `src/convert.rs::test_gps_utc_roundtrip_exact_at_nanosecond_level` | `tests/prop_tests.rs::prop_gps_utc_gps_roundtrip_exact`; `fuzz_gps_utc.rs`/`fuzz_utc_to_gps.rs` |
| I-16 Leap-second window          | `src/convert.rs::test_gps_to_utc_detects_leap_second_ambiguity`, `src/leap.rs::test_leap_second_transition_1999_gps_jumps_by_2s`, `::test_leap_second_transition_2017_gps_jumps_by_2s` | `tests/prop_tests.rs::prop_ambiguous_only_near_boundaries`, `::prop_gps_near_leap_converts_consistently`; `fuzz_gps_utc.rs`/`fuzz_utc_to_gps.rs` BOUNDARY mode |
| I-17 No heap allocation          | `tests/no_std_compact.rs::test_no_heap_allocation_in_conversions` | CI: `no-std-transitive` job in `.github/workflows/embedded.yml` |
| I-18 8-byte size                 | `src/time.rs::test_size_equals_u64` | CI: `type-sizes` job; `firmware/` size probe |
| I-19 No unsafe code              | `rustc` (`#![forbid(unsafe_code)]`) | — |
| I-20 No missing docs             | `rustc` (`#![deny(missing_docs)]`), `cargo clippy --all-targets -- -D warnings` | — |

Additional fuzz-only coverage not tied to a single numbered invariant above:

- **`fuzz_try_extend.rs`** — pins the full error-priority contract of
  `RuntimeLeapSeconds::try_extend` (`BufferFull` >
  `NotStrictlyAscending`/`NonUnitIncrement`/`OffsetOverflow`) against an
  independent `i64` reference model; this is the harness that found and pinned
  the `i32::MAX` offset-wraparound fix (`OffsetOverflow`), documented in
  `CHANGELOG.md`.
- **`fuzz_leap_lookup.rs`** — monotonicity and dynamic-range (`min..=max` of
  the *actual* table, not a hardcoded `19..=37`) of `tai_minus_utc_at` for
  both `LeapSeconds` and `RuntimeLeapSeconds`, including the empty-table
  fallback.
- **`fuzz_week_tow.rs`/`fuzz_day_tod.rs`** — exact error classification
  (`InvalidInput` vs. `Overflow` vs. `Ok`) and field-roundtrip exactness for
  `from_week_tow`/`from_day_tod` across their full input domain.

See `fuzz/README.md` for how to run these locally and the current
zero-crash status.
