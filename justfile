# The gnss-time development commands (Justfile)
#
# Purpose:
#   Unified interface for formatting, linting, testing, documentation,
#   embedded validation, feature-matrix checks, and CI simulation.
#
# Usage:
#   just <recipe>

set shell := ["bash", "-ceuo", "pipefail"]

# =============================================================================
# Default
# =============================================================================

default:
    just help

help:
    @just --list

# =============================================================================
# Toolchain / Targets
# =============================================================================

setup-embedded:
    rustup target add thumbv7em-none-eabihf
    rustup target add thumbv6m-none-eabi

setup-riscv:
    rustup target add riscv32imac-unknown-none-elf
    rustup target add riscv32i-unknown-none-elf

# =============================================================================
# Formatting
# =============================================================================

fmt:
    cargo fmt --all
    taplo fmt

fmt-toml:
    taplo fmt

fmt-all: fmt fmt-toml

fmt-check:
    cargo fmt --all -- --check
    taplo fmt --check

# =============================================================================
# Cargo checks
# =============================================================================

check:
    cargo check --workspace --all-targets --locked

check-all-features:
    cargo check --workspace --all-features --locked

check-std:
    cargo check --workspace --features std --locked

# =============================================================================
# Embedded / no_std validation
# =============================================================================

check-no-std: setup-embedded
    cargo check --lib --no-default-features --target thumbv7em-none-eabihf --locked

check-no-std-defmt: setup-embedded
    cargo check --lib --no-default-features --features defmt --target thumbv7em-none-eabihf --locked

# Cortex-M0/M0+ (STM32F0xx, nRF51).

check-no-std-cortex-m0: setup-embedded
    cargo check --lib --no-default-features --target thumbv6m-none-eabi --locked

# RISC-V targets (RV32IMAC with atomics, RV32I without).

check-riscv: setup-riscv
    cargo check --lib --no-default-features --target riscv32imac-unknown-none-elf --locked
    cargo check --lib --no-default-features --target riscv32i-unknown-none-elf --locked

# =============================================================================
# Embedded size report
# =============================================================================

setup-size:
    cargo install cargo-binutils --locked
    rustup component add llvm-tools-preview

# Build the Cortex-M probe firmware and print section + symbol sizes.
#
# Requires cargo-binutils + llvm-tools-preview (see `just setup-size`).

size:
    cargo build --release --manifest-path firmware/Cargo.toml --target thumbv7em-none-eabihf
    cargo size --release --manifest-path firmware/Cargo.toml --target thumbv7em-none-eabihf -- -A
    cargo bloat --release --manifest-path firmware/Cargo.toml --target thumbv7em-none-eabihf -n 15

# =============================================================================
# Linting
# =============================================================================

lint:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

lint-no-std: setup-embedded
    cargo clippy --lib --no-default-features --features defmt --target thumbv7em-none-eabihf --locked -- -D warnings

# =============================================================================
# Documentation
# =============================================================================

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked

docsrs:
    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --workspace --all-features --no-deps

# =============================================================================
# MSRV validation
#
# Mirrors the `msrv` job in .github/workflows/msrv.yml: same feature matrix,
# same `--locked`-against-the-committed-v3-lockfile strategy. Uses an explicit
# `+1.75.0`: the only rustup override level that outranks the
# `nightly-2026-02-14` pinned by rust-toolchain.toml.
# =============================================================================

msrv:
    cargo +1.75.0 check --workspace --lib --no-default-features --locked
    cargo +1.75.0 check --workspace --lib --features std --locked
    cargo +1.75.0 check --workspace --lib --no-default-features --features defmt --locked
    cargo +1.75.0 check --workspace --lib --no-default-features --features serde --locked
    cargo +1.75.0 check --workspace --lib --all-features --locked

# =============================================================================
# Feature matrix validation
# =============================================================================

hack:
    cargo hack check --workspace --feature-powerset --no-dev-deps

hack-each:
    cargo hack check --workspace --each-feature

# =============================================================================
# Tests
# =============================================================================
#
# Run the full test suite (unit + integration + determenistic property tests).
# proptest-based tests in prop_test.rs are compiled automatically on host

test:
    cargo test --workspace --all-features --locked

# Deterministic property tests.
#
# These tests do not require the std feature and are intended to run
# identically across all host environments.

test-deterministic:
    cargo test --test prop_deterministic --locked

# Proptest-based randomized property tests.
#
# Explicit target for CI jobs that isolate randomized/property testing.

test-props:
    cargo test --features std --test prop_tests --locked

test-serde:
    cargo test --features serde

# Complete test suite.

test-all:
    just test
    just test-deterministic
    just test-props
    just test-serde

# Bare-metal smoke test.
#
# Actual execution is not possible on embedded targets in CI, but we verify
# that the crate builds successfully in no_std mode.

test-no-std: setup-embedded
    cargo check --lib --no-default-features --target thumbv7em-none-eabihf --locked

next:
    cargo nextest run --workspace

# =============================================================================
# Benchmarks
# =============================================================================

bench:
    cargo bench -p benches --locked

bench-smoke:
    cargo bench -p benches --locked -- --test

# =============================================================================
# Fuzzing
# =============================================================================

setup-fuzz:
    cargo install cargo-fuzz --locked

# Build all fuzz targets (requires nightly, see rust-toolchain.toml).

fuzz-build:
    cargo fuzz build

# Run all fuzz targets (long local campaign; default 5 minutes per target).
#
# Usage: just fuzz [secs=300]

fuzz secs='300':
    @for t in fuzz_week_tow fuzz_day_tod fuzz_gps_utc fuzz_utc_to_gps fuzz_try_extend fuzz_leap_lookup; do cargo fuzz run "$t" -- -max_total_time={{ secs }}; done

# =============================================================================
# Advanced validation
# =============================================================================

udeps:
    cargo +nightly udeps --all-targets --all-features

miri:
    cargo +nightly miri test

deny:
    cargo deny check

audit:
    cargo audit

release-check:
    cargo publish --dry-run

# ════════════════════════════════════════════════════════════════════════════
# Release (Issue #TIME-36)
#
# Releases are driven by release-plz: push to main → release-plz opens a
# Release PR → you merge it → release-plz tags, publishes to crates.io, and
# creates the GitHub Release.
#
# These recipes verify and preview that process locally. They do not publish.
# ════════════════════════════════════════════════════════════════════════════

# Install the release tooling (one-time setup)
install-release-tools:
    cargo install release-plz --locked
    cargo install cargo-semver-checks --locked
    @echo "✓ release-plz + cargo-semver-checks installed"

# Preview the Release PR release-plz would open: next version and changelog.
#
# Read-only — makes no commits, no tags, no network writes.
release-preview:
    release-plz update --dry-run

# Show the changelog entry that would be generated for the next release.
release-changelog:
    release-plz changelog

# Check the public API against the last published version on crates.io.
#
# Mirrors the semver-checks CI job. Both feature sets are checked because a
# feature-gated item is still public API to anyone enabling that feature.
# Requires the crate to already be published on crates.io (it is, as of
# v0.9.1), which is also why this recipe is not part of `just ci`:
# offline runs must not fail.
semver-check:
    @echo "── default features ────────────────────────────────────────────"
    cargo semver-checks check-release --only-explicit-features
    @echo "── all features ────────────────────────────────────────────────"
    cargo semver-checks check-release --all-features
    @echo "✓ semver check passed"

# Verify the crate packages cleanly for crates.io.
#
# This catches the failure mode the explicit `include` list in Cargo.toml
# makes possible: a file present locally but omitted from the published
# .crate archive, so the crate builds for you and fails for everyone else.
package-check:
    cargo publish --dry-run
    @echo "── files that would be published ───────────────────────────────"
    cargo package --list

# Full pre-release gate: everything CI runs on the Release PR, plus a preview.
#
# Run this before pushing the commits you intend to release. Note there is no
# separate `doctest` recipe in this justfile — doctests already run as part of
# `test-all` (cargo test compiles them).
release: lint test-all msrv semver-check package-check release-preview
    @echo ""
    @echo "✓ Pre-release checks passed."
    @echo ""
    @echo "  Next: push to main. release-plz will open a Release PR."
    @echo "  Merge that PR to publish."

# Escape hatch: publish manually, bypassing release-plz.
#
# Only for when the automation is broken. Prefer the Release PR: this skips
# the semver gate, the generated changelog, and the CI run that the PR
# would have had.
#
# Requires CARGO_REGISTRY_TOKEN in the environment.
release-publish: release
    @echo ""
    @echo "⚠  Publishing manually — bypassing release-plz."
    @read -p "Type the version being published to confirm: " v; \
      test "$v" = "$(grep '^version' Cargo.toml | head -1 | sed 's/.*= *"\(.*\)"/\1/')" \
      || { echo "Version mismatch — aborting."; exit 1; }
    cargo publish

# =============================================================================
# CI aggregate
# =============================================================================

ci: fmt-check lint check check-std check-no-std check-no-std-defmt msrv doc hack test-all bench-smoke

# =============================================================================
# Cleanup
# =============================================================================

clean:
    cargo clean
