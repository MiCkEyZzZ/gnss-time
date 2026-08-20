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

| Operation                                   | Time     | Note                                     |
| ------------------------------------------- | -------- | ---------------------------------------- |
| `Time<Gps> + Duration` (panicking)          | ~507 ps  | matches `u64 + u64` within noise         |
| `u64 + u64` (baseline)                      | ~505 ps  | baseline addition                        |
| `Time<Gps> - Time<Gps>` (panicking)         | ~505 ps  | matches `u64 - u64` within noise         |
| `u64 - u64` (baseline)                      | ~505 ps  | baseline subtraction                     |
| `Time<Gps>.checked_add`                     | ~4.30 ns | with overflow checking                   |
| `Time<Gps>.checked_sub_duration`            | ~4.29 ns | with underflow checking                  |
| `Time<Gps>.saturating_add`                  | ~505 ps  | no measurable extra cost                 |
| `Time<Gps>.saturating_add` (at `MAX`)       | ~506 ps  | constant-time clamp                      |
| `Duration + Duration`                       | ~506 ps  | matches raw arithmetic                   |
| `Duration.checked_add`                      | ~4.28 ns | with overflow checking                   |

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

### Time primitives (`time_bench`)

| Operation                | Time     | Note                                      |
| ------------------------ | -------- | ----------------------------------------- |
| `u64` add                | ~1.06 ns | baseline (separate harness from above)    |
| `Time<Gps> + Duration`   | ~1.26 ns | typed add                                 |
| `Time<Gps> - Duration`   | ~1.02 ns | typed subtract                            |
| `Time<Gps>` diff         | ~1.26 ns | `Time - Time`                             |
| `Time::from_nanos`       | ~508 ps  | constructor                               |
| `Time<Gps> → TAI`        | ~796 ps  | fixed +19 s conversion                   |

**Note:** `time_bench` and `arithmetic_bench` both exercise add/sub; numbers
differ slightly because Criterion groups and black-box patterns are separate.
Use `arithmetic_bench` for zero-cost vs `u64` claims; use `time_bench` for
typed API micro-costs.

## Zero-cost abstraction check

Within `arithmetic_bench`, typed panicking arithmetic matches the `u64`
baseline within measurement noise:

| Pair                                      | Typed   | Raw `u64` | Δ      |
| ----------------------------------------- | ------- | --------- | ------ |
| `Time + Duration` vs `u64 + u64`          | ~507 ps | ~505 ps   | ~2 ps  |
| `Time - Time` vs `u64 - u64`              | ~505 ps | ~505 ps   | ~0 ps  |
| `saturating_add` (normal)                 | ~505 ps | —         | ≈ raw  |
| `checked_add` / `checked_sub_duration`    | ~4.3 ns | —         | expected branch cost |

## CI

CI runs a smoke-check (`cargo bench -p benches -- --test`) to ensure
benchmarks compile and execute. Full Criterion timing runs are local-only —
they need a quiet host and are not used as pass/fail gates.
