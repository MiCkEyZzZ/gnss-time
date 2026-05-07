# Benchmarks for gnss-time

This is directory contains benchmarks used to verify zero-cost abstractions and
the performance of time conversions.

## Running

```bash
cargo bench
```

Or run individual benchmark groups:

```bash
cargo bench --bench arithmetic_bench
cargo bench --bench convert_bench
cargo bench --bench time_bench
```

## Results

### Arithmetic

| Operation                                   | Time     | Note                                     |
| ------------------------------------------- | -------- | ---------------------------------------- |
| `Time<Gps> + Duration` (panicking)          | ~505 ps  | 0 ns overhead                            |
| `u64 + u64` (baseline)                      | ~504 ps  | baseline addition                        |
| `Time<Gps> - Time<Gps>` (panicking)         | ~504 ps  | 0 ns overhead                            |
| `u64 - u64`                                 | ~505 ps  | baseline subtraction                     |
| `Time<Gps>.checked_add`                     | ~4.29 ns | with overflow checking                   |
| `Time<Gps>.checked_sub_duration`            | ~4.27 ns | with underflow checking                  |
| `Time<Gps>.saturating_add`                  | ~505 ps  | no extra checks                          |
| `Time<Gps>.saturating_add (at MAX, clamps)` | ~509 ps  | saturated edge case, constant-time clamp |
| `Duration + Duration`                       | ~506 ps  | 0 ns overhead                            |
| `Duration.checked_add`                      | ~4.28 ns | with checking                            |

**Conclusion:** panicking operations have no measurable overhead. Checked
operations add a small cost (< 5 ns).

### Conversions

| Operation                                | Time     | Target  |
| ---------------------------------------- | -------- | ------- |
| `GPS → TAI`                              | ~807 ps  | < 2 ns  |
| `GPS → Galileo`                          | ~764 ps  | < 2 ns  |
| `GPS → BeiDou`                           | ~874 ps  | < 2 ns  |
| `TAI → GPS`                              | ~778 ps  | < 2 ns  |
| `GPS → UTC` (table lookup, 2020)         | ~9.6 ns  | < 10 ns |
| `GPS → UTC` (GPS epoch)                  | ~9.6 ns  | < 10 ns |
| `UTC → GPS` (two-pass algorithm)         | ~22.0 ns | —       |
| `GPS → UTC → GPS` (roundtrip)            | ~39.8 ns | —       |
| `LeapSeconds` binary search (19 entries) | ~6.9 ns  | —       |

**Conclusion:** fixed-offset conversions are effectively free (~0.8–0.9 ns).
Leap-second-aware conversions (< 10 ns for GPS → UTC) are suitable for
embedded and high-throughput systems.

## Zero-cost abstraction check

The benchmarks show that `Time<Gps> + Duration` compiles to the same
instructions as `u64 + u64`:

- Panicking operations: **514 ps** vs **514 ps** (0 ns difference)
- `saturating_add`: **515 ps** (clamp without extra checks)
- Checked operations are more expensive (~4.4 ns), but that is expected
  (one branch + overflow check)

## CI

Benchmarks are not run automatically in CI (they take time and require a stable
environment). Run them locally before a release and when refactoring critical
paths.
