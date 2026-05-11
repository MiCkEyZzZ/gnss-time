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
# =============================================================================

msrv:
    cargo +1.75.0 check --workspace --lib --no-default-features
    cargo +1.75.0 check --workspace --lib --features std
    cargo +1.75.0 check --workspace --lib --no-default-features --features defmt

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

# =============================================================================
# CI aggregate
# =============================================================================

ci: fmt-check lint check check-std check-no-std check-no-std-defmt msrv doc hack test-all test-serde

# =============================================================================
# Cleanup
# =============================================================================

clean:
    cargo clean
