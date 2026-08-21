# Benchmarks for gnss-time

This directory contains benchmarks used to verify zero-cost abstractions and
the performance of time conversions.

## Running

From the repository root:

```bash
just bench
```

Or from the `benches/` crate directory:

```bash
cd benches
cargo bench
```

Individual benchmark groups:

```bash
cd benches
cargo bench --bench arithmetic_bench
cargo bench --bench convert_bench
cargo bench --bench time_bench
cargo bench --bench providers_bench
```

Smoke-check (compile and run without collecting timings):

```bash
just bench-smoke
# or: cargo bench -p benches --locked -- --test
```

## Results

Figures below are Criterion mid estimates from a local host run of
`cargo bench -p benches`. Absolute times vary by CPU and load; relative
comparisons within a single run are what matter.

### Arithmetic (`arithmetic_bench`)

| Operation                                   | Time     | Note                               |
| ------------------------------------------- | -------- | ---------------------------------- |
| `Time<Gps> + Duration` (panicking)          | ~507 ps  | matches `u64 + u64` within noise   |
| `u64 + u64` (baseline)                      | ~505 ps  | baseline addition                  |
| `Time<Gps> - Time<Gps>` (panicking)         | ~505 ps  | matches `u64 - u64` within noise   |
| `u64 - u64` (baseline)                      | ~505 ps  | baseline subtraction               |
| `Time<Gps>.checked_add`                     | ~4.30 ns | with overflow checking             |
| `Time<Gps>.checked_sub_duration`            | ~4.29 ns | with underflow checking            |
| `Time<Gps>.saturating_add`                  | ~505 ps  | no measurable extra cost           |
| `Time<Gps>.saturating_add` (at `MAX`)       | ~506 ps  | constant-time clamp                |
| `Duration + Duration`                       | ~506 ps  | matches raw arithmetic             |
| `Duration.checked_add`                      | ~4.28 ns | with overflow checking             |

**Conclusion:** panicking / saturating arithmetic has no measurable overhead
versus raw `u64` ops. Checked paths add ~4 ns (branch + overflow check).

### Conversions (`convert_bench`)

| Operation                                      | Time     | Target  |
| ---------------------------------------------- | -------- | ------- |
| `GPS → TAI` (fixed +19 s)                      | ~808 ps  | < 2 ns  |
| `GPS → Galileo` (identity)                     | ~773 ps  | < 2 ns  |
| `GPS → BeiDou` (fixed −14 s via TAI)           | ~869 ps  | < 2 ns  |
| `TAI → GPS` (fixed −19 s)                      | ~1.02 ns | < 2 ns  |
| `GPS → UTC` (builtin table, 2020)              | ~9.01 ns | < 10 ns |
| `GPS → UTC` (builtin table, GPS epoch 1980)    | ~9.00 ns | < 10 ns |
| `UTC → GPS` (two-pass algorithm, 2020)         | ~22.6 ns | —       |
| `GPS → UTC → GPS` (full roundtrip)             | ~36.8 ns | —       |
| `LeapSeconds::builtin` binary search (19 rows) | ~7.04 ns | —       |

**Conclusion:** fixed-offset conversions stay ~0.8–1.0 ns. Leap-second-aware
`GPS → UTC` stays under 10 ns; roundtrips are dominated by UTC resolution.

#### `ConvertResult` overhead (`convert_bench`)

| Operation                                        | Time     | Note                                    |
| ------------------------------------------------ | -------- | --------------------------------------- |
| `gps_to_utc` (no wrapper, baseline)              | ~9.02 ns | raw contextual conversion               |
| `into_scale_with_checked` (+ `ConvertResult`)    | ~25.7 ns | adds ambiguity-window detection         |
| `ConvertResult::is_exact()`                      | ~504 ps  | enum match, folds to a flag test        |
| `ConvertResult::into_inner()`                    | ~506 ps  | enum match                              |

**Conclusion:** the `ConvertResult` wrapper itself costs ~0.5 ns per access —
the enum is free once values are inlined. The checked path is ~2.8× the raw
conversion because it performs extra leap-second lookups to detect the
ambiguity window, not because of the enum.

### Time primitives (`time_bench`)

| Operation                | Time     | Note                                    |
| ------------------------ | -------- | --------------------------------------- |
| `u64` add                | ~1.06 ns | baseline (separate harness from above)  |
| `Time<Gps> + Duration`   | ~1.26 ns | typed add                               |
| `Time<Gps> - Duration`   | ~1.02 ns | typed subtract                          |
| `Time<Gps>` diff         | ~1.26 ns | `Time - Time`                           |
| `Time::from_nanos`       | ~508 ps  | constructor                             |
| `Time::from_week_tow`    | ~2.02 ns | constructor with TOW validation         |
| `Time<Gps> → TAI`        | ~796 ps  | fixed +19 s conversion                  |

**Note:** `time_bench` and `arithmetic_bench` both exercise add/sub; numbers
differ slightly because Criterion groups and black-box patterns are separate.
Use `arithmetic_bench` for zero-cost vs `u64` claims; use `time_bench` for
typed API micro-costs.

### Leap-second context (`providers_bench`)

Same instant (2020-01-06), different [`LeapSecondsProvider`] implementations.

`GPS → UTC` through a provider:

| Provider                                  | Time     | Note                              |
| ----------------------------------------- | -------- | --------------------------------- |
| `LeapSeconds::builtin()` (static table)   | ~9.63 ns | binary search over 19 entries     |
| `RuntimeLeapSeconds` (19 entries)         | ~9.84 ns | heap-free buffer, same search     |
| `RuntimeLeapSeconds` (empty, fallback)    | ~2.29 ns | early return of fallback value    |
| custom constant (receiver-style)          | ~1.27 ns | no table lookup at all            |

Direct `tai_minus_utc_at` lookup:

| Provider                                  | Time     |
| ----------------------------------------- | -------- |
| `LeapSeconds::builtin()` (static table)   | ~6.99 ns |
| `RuntimeLeapSeconds` (19 entries)         | ~7.24 ns |
| `RuntimeLeapSeconds` (empty, fallback)    | ~1.79 ns |
| custom constant (receiver-style)          | ~764 ps  |

**Conclusion:** the runtime fixed-capacity table matches the static table
within noise (~7 ns per lookup). A receiver that knows the current offset
from the navigation message and passes it as a constant skips the search
entirely (~0.8 ns).

### UTC → GPS: two-pass vs one-pass (`providers_bench`)

| Algorithm                          | Time      | Note                                   |
| ---------------------------------- | --------- | -------------------------------------- |
| two-pass (public `utc_to_gps`)     | ~21.98 ns | two binary searches + refinement       |
| one-pass reference (single lookup) | ~8.83 ns  | baseline: what one search would cost   |

**Conclusion:** the second pass costs ~13 ns (~2.5×). It is the price of
correct leap-second boundary handling in the public API; the one-pass variant
is provided as a measurement baseline only and is not part of the API.

## Zero-cost abstraction check

Within `arithmetic_bench`, typed panicking arithmetic matches the `u64`
baseline within measurement noise:

| Pair                                      | Typed   | Raw `u64` | Δ                    |
| ----------------------------------------- | ------- | --------- | -------------------- |
| `Time + Duration` vs `u64 + u64`          | ~507 ps | ~505 ps   | ~2 ps                |
| `Time - Time` vs `u64 - u64`              | ~505 ps | ~505 ps   | ~0 ps                |
| `saturating_add` (normal)                 | ~505 ps | —         | ≈ raw                |
| `checked_add` / `checked_sub_duration`    | ~4.3 ns | —         | expected branch cost |

## Comparison with hifitime (`vs_hifitime`)

Head-to-head against [`hifitime`](https://crates.io/crates/hifitime) 4.x for
semantically equivalent operations. Figures are Criterion mid estimates from
the same host run, so relative differences are directly comparable.

| Operation      | gnss-time | hifitime | Ratio |
| -------------- | --------- | -------- | ----- |
| construct GPS  | ~507 ps   | ~7.34 ns | ~14×  |
| add duration   | ~506 ps   | ~7.34 ns | ~14×  |
| GPS → TAI      | ~807 ps   | ~10.2 ns | ~13×  |
| GPS → UTC      | ~9.66 ns  | ~26.2 ns | ~2.7× |

**Type sizes:** `Time<S>` is 8 bytes; `hifitime::Epoch` is 24 bytes
(`i16` centuries + `u64` ns + `TimeScale`).

**Interpretation:** gnss-time's fixed-offset paths (`construct`, `add`,
`GPS → TAI`) compile down to a single integer op, giving ~13–14× advantage.
The leap-second-aware `GPS → UTC` path is the closest race: both need a table
lookup, and gnss-time still wins by ~2.7×. The gap widens as the operation
moves closer to raw arithmetic; it narrows on operations dominated by
leap-second data. hifitime's extra cost buys a richer model (time scale stored
in the type, century-wide range, TDB/ET/TT support) at 3× the memory footprint.

**Methodology notes:**

- Both crates benchmarked with the same `black_box` discipline in one Criterion
  run.
- Construction differs structurally: gnss-time takes integer week/TOW;
  hifitime takes `f64` seconds since GPS epoch.
- hifitime is compiled with `default-features = false` (no `std`) for parity
  with gnss-time's `no_std` promise.

## CI

The `bench` job in `ci.yml` does two things:

1. Smoke-check (`cargo bench -p benches -- --test`) — benchmarks compile and
   execute, no timing assertions.
2. A measured run with reduced Criterion timing
   (`--measurement-time 1 --warm-up-time 0.3 --sample-size 20`). Raw Criterion
   output (`target/criterion/`) is uploaded as the `benchmark-results`
   artifact (retained 30 days).

CI numbers come from shared runners and are noisy; use them for trend
detection only. Full-fidelity runs are local — they need a quiet host and are
not used as pass/fail gates.
