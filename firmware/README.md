# firmware — `.text` size probe

`gnss-time-size-probe` — a bare-metal crate (thumbv7em-none-eabihf) for
measuring the code size of `gnss-time` in a release build. Each operation is
isolated into its own `#[inline(never)]` symbol guarded by `black_box`, so it
can be measured independently. It is used by the `size-report` CI job
(`.github/workflows/embedded.yml`) for Issue #TIME-26.

## Why it is not a workspace member

The crate only builds for a bare-metal target (`cortex-m-rt`, linker script,
its own `#[panic_handler]`) and must not break host builds of
`cargo check --workspace` — so it is **not** listed in the root `Cargo.toml`
`members`, and its own empty `[workspace]` table opts it out of the root
workspace. CI builds it separately via
`--manifest-path firmware/Cargo.toml`.

## Prerequisites

```sh
rustup target add thumbv7em-none-eabihf
rustup component add llvm-tools-preview
cargo install cargo-binutils --locked
```

(alternatively: `just setup-size`)

## Build & measure

```sh
cargo build --release --target thumbv7em-none-eabihf
cargo size   --release --target thumbv7em-none-eabihf -- -A
cargo bloat  --release --target thumbv7em-none-eabihf -n 100
```

(or `just size`)

## Current results (release, thumbv7em-none-eabihf)

| Metric | Value |
| ------ | ----- |
| `.text` (whole binary) | 980 B |
| `.rodata` | 388 B |
| `.data` / `.bss` / `.uninit` | 0 B |

Measured symbols:

| Symbol | `.text` |
| ------ | ------- |
| `Time<Gps>::from_week_tow` | 182 B |
| `probe_gps_to_utc` | 180 B |
| `LeapSeconds::tai_minus_utc_at` | 138 B |
| `__cortex_m_rt_main` | 144 B |
| `Time<Gps>::to_tai` | 56 B |
| `probe_time_checked_add` | 56 B |
| `Reset` | 62 B |
| `probe_time_saturating_add` | 42 B |
| `probe_from_week_tow` | 34 B |
| `probe_into_scale` | 32 B |

## Design notes

- The probes **deliberately avoid** `unwrap()`/`panic!` and the panicking
  operators (`+`/`-`): this is a size probe, not a user application, so it must
  not pull in the panic/`core::fmt` formatting machinery. On error a probe just
  halts (`Err(_) => loop {}`).
- A panicking `+`/`-` operator would add ~1.9 KiB of formatting infrastructure
  to `.text` (`do_count_chars` 1.2 KiB, `Formatter::pad` 466 B, ...);
  `unwrap()` pulls in even more (the resulting `.text` was 9516 B). For embedded
  code prefer `saturating_add` / `checked_add` / `try_add` instead.
- `black_box` prevents constant folding and dead-code elimination.
- `memory.x` is a minimal linker script (256K flash / 64K RAM, STM32-style
  addresses); no real board is required.

## Layout

```
firmware/
├── Cargo.toml   — package + empty [workspace] (outside the root workspace)
├── memory.x     — linker script
└── src/main.rs  — #[panic_handler], probes, cortex-m-rt entry point
```