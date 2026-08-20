# Release notes — gnss-time v0.6.0

**Date:** 2026-08-20

**Summary:** embedded size verification (Issue #TIME-26), extended embedded
target coverage, `defmt::Format` for `Duration`, golden postcard wire-format
tests, and a standard crate landing page in the README.

## Added

- **`no_alloc` / embedded size verification (Issue #TIME-26):**
  - `examples/embedded_minimal.rs` — dual-mode example: runs as a host binary
    and compiles as a `no_std`/`no_main` Cortex-M binary.
  - `firmware/` — standalone `thumbv7em-none-eabihf` size-probe crate with one
    `#[inline(never)]` symbol per core operation; clean firmware `.text` = 980 B.
  - `just size` / `just setup-size` recipes (`cargo size -A` + `cargo bloat`).
  - `size-report` CI job enforcing a `.text` budget (< 2 KiB) and verifying that
    `Time + Duration` compiles to a native 64-bit add (`adds`/`adcs`).
  - Per-symbol `.text` sizes documented in `docs/EMBEDDED.md`.
- **Extended embedded target coverage:** `thumbv6m-none-eabi` (Cortex-M0/M0+)
  and `riscv32i-unknown-none-elf` added to the CI matrix, plus a `no_std`
  transitive-dependency check (`cargo tree -e normal`).
- **Head-to-head benchmark against `hifitime` 4.x** (`benches/`).
- **`impl defmt::Format for Duration`** (was documented but missing), formatted
  as `"Xs Yns"` like `Display`; compile-verified for every embedded target.
- **Golden postcard wire-format tests** (`serde_impls::tests::*postcard_golden`)
  pinning exact byte sequences for `Time<S>`, `Duration` and `DurationParts`.

## Changed

- `README.md` restructured into a standard crate landing page: quick start,
  installation, feature-flags table, usage, time scale model, safety model,
  performance, documentation links, MSRV, contributing and license.
- `docs/EMBEDDED.md` refined after review:
  - In-memory (8 B) vs wire (variable-length) representation distinguished.
  - Embedded "safe arithmetic" example rewritten without `unwrap()`.
  - Panic behaviour clarified to depend on the `#[panic_handler]` (probe
    firmware uses an infinite loop), replacing "panic = abort".
  - `.text` = 980 B and `adds`/`adcs` claims refined (Cortex-M-specific).
  - UBX parsing example uses array indexing instead of `try_into().unwrap()`.
- Dependabot extended to the `cargo` ecosystem with conventional-commit
  prefixes, grouping, labels and `target-branch: main`.

## Fixed

- Incorrect ULEB-128 byte sequence for
  `DurationParts { seconds: 5, nanos: 500_000_000 }` in `docs/EMBEDDED.md`
  (now `[0x80, 0xCA, 0xB5, 0xEE, 0x01]`, enforced by golden tests).
- Missing `defmt::Format` implementation for `Duration`.

## Compatibility

No breaking API changes. MSRV remains **Rust 1.75.0**.

Full details: [CHANGELOG.md](CHANGELOG.md).