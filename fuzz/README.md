# Fuzzing `gnss-time`

Coverage-guided fuzzing harnesses for the [`gnss-time`](../README.md) crate,
built on [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) /
[`libFuzzer`](https://llvm.org/docs/LibFuzzer.html) via the
`libfuzzer-sys` crate. Each target verifies a small set of documented
invariants (roundtrip exactness, monotonicity, error classification) on top
of the no-panic guarantee.

Every target follows the same dual-mode layout: a **RAW** mode samples the
full `u64` input domain uniformly, and a **BOUNDARY** mode selects one of the
library's real validity edges (leap-second transitions, constructor rims,
capacity limits) from a compact selection byte so each edge is provably
exercised on **every** run — blind mutation of the full `u64` space can never
reliably reach a 2-second ambiguity window or an overflow rim.

## Targets

| Target               | Crate under test                                                 | What it checks                                                                                                                | `max_len` |
| -------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | --------- |
| `fuzz_gps_utc`       | `Time::<Gps>` ↔ `Time::<Utc>`                                    | Roundtrip exactness (I-12), no-panic on any `u64`, monotonicity, TAI − UTC step bounds                                        | 12        |
| `fuzz_utc_to_gps`    | `Time::<Utc>` → `Time::<Gps>`                                    | Mirror of `fuzz_gps_utc` in the UTC domain, GPS-epoch underflow boundary                                                      | 12        |
| `fuzz_week_tow`      | `Time::<Gps>::from_week_tow`                                     | Error classification (`InvalidInput`/`Overflow`/`Ok`), exact arithmetic, field roundtrips                                     | 17        |
| `fuzz_day_tod`       | `Time::<Glonass>::from_day_tod`                                  | Mirror of `fuzz_week_tow` for GLONASS day/TOD, `day_of_week()` range consistency                                              | 17        |
| `fuzz_try_extend`    | `RuntimeLeapSeconds::try_extend` / `LeapSeconds::try_from_slice` | Contract against an `i64` reference model: `Ok`/`BufferFull`/`NotStrictlyAscending`/`NonUnitIncrement`/`OffsetOverflow`       | 793       |
| `fuzz_leap_lookup`   | `tai_minus_utc_at` (runtime + static `builtin()`)                | Monotone non-decreasing offset, dynamic `min..=max` range, 19 s empty-table fallback                                          | 922       |

Each target is registered as a `[[bin]]` in [`Cargo.toml`](Cargo.toml) with
`test = false` and lives in [`fuzz_targets/`](fuzz_targets/). The full input
layout is documented at the top of each harness.

## Prerequisites

```sh
rustup toolchain install nightly
rustup target add x86_64-unknown-linux-gnu   # libFuzzer runs on Linux only
cargo install cargo-fuzz --locked
```

## Running

From the repository root (all recipes live in the
[`justfile`](../justfile)):

```sh
just setup-fuzz     # nightly + cargo-fuzz bootstrap
just fuzz-build     # compile all targets once
just fuzz 300       # run every target for 300 s each
```

Or a single target, directly from this directory with the recommended
`-max_len` and dictionary already applied:

```sh
cd fuzz
./tools/run-fuzz.sh fuzz_gps_utc                        # default 300 s
MAX_TOTAL_TIME=600 ./tools/run-fuzz.sh fuzz_leap_lookup  # longer campaign
./tools/run-fuzz.sh fuzz_week_tow -runs=100000           # fixed run count
cargo fuzz run fuzz_try_extend -- -max_total_time=300 -max_len=793 \
    -dict=fuzz_try_extend.dict
```

A successful short smoke run ends with a line like
`Done 6000000 runs in 20 second(s)` and **no** `crash-*` files.

## Seed corpus

Because RAW spaces are astronomically large, fuzzing starts from a curated
seed corpus that already sits on the edges libFuzzer must discover.

- Per-target edge seeds (one per validity edge, **no** cross-products) plus
  boundary jitter walks that cross each edge by ±1/±2/± the jitter window.
- `./tools/gen_corpus.sh` regenerates all corpora deterministically and
  reports their sizes (`fuzz_gps_utc`: 407, `fuzz_week_tow`: 137,
  `fuzz_day_tod`: 134, `fuzz_utc_to_gps`: 421, `fuzz_try_extend`: 19,
  `fuzz_leap_lookup`: 166).

`corpus/` and `artifacts/` are git-ignored (see [`.gitignore`](.gitignore));
regenerate seeds with `./tools/gen_corpus.sh` and inspect crashes under
`artifacts/`.

## Findings

- **`i32::MAX` leap-offset wraparound** (found by `fuzz_try_extend`): a first
  entry with `tai_minus_utc = i32::MAX` caused `try_extend`/`try_from_slice`
  to compute `last + 1` in `i32`, panicking in debug builds and silently
  accepting an `i32::MIN` successor in release. Fixed with
  `checked_add(1).ok_or(LeapExtendError::OffsetOverflow)`; both paths now
  return the new `OffsetOverflow` variant.

All six targets currently run clean (millions of executions, zero crashes).

## Crash triage

On a crash, `libFuzzer` writes `artifacts/<target>/crash-<hash>`:

```sh
cargo fuzz run fuzz_try_extend artifacts/fuzz_try_extend/crash-<hash>   # reproduce
cargo fuzz tmin fuzz_try_extend artifacts/fuzz_try_extend/crash-<hash>  # minimize
cargo fuzz run fuzz_try_extend artifacts/fuzz_try_extend/crash-<hash> -- -shrink=1
```

Before reporting a library bug, rule out a **harness bug** (an assertion in
the fuzz target itself rather than in `gnss-time`). The input layout comments
in each harness and the `ExpectedKind`/reference-model approach exist precisely
to keep harness-internal failures distinguishable from crate defects.

## Adding a target

1. Add `fuzz_targets/<name>.rs` following the RAW/BOUNDARY split and the
   property-check (not re-implemented-oracle) style of the existing harnesses.
2. Register it in [`Cargo.toml`](Cargo.toml); add `<name>.dict` and a
   `max_len`/dict entry in [`tools/run-fuzz.sh`](tools/run-fuzz.sh).
3. Add seeds + a counter line in [`tools/gen_corpus.py`](tools/gen_corpus.py)
   and [`tools/gen_corpus.sh`](tools/gen_corpus.sh).
4. Extend the target table above, the `fuzz` recipe in the `justfile`, and the
   `CHANGELOG.md` entry.
