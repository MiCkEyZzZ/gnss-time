# The gnss-time development commands (Justfile)
#
# Purpose:
#   Unified command interface for build, lint, test, formatting,
#   embedded targets, and CI simulation.
#
# Usage:
#   just <recipe>

set shell := ["bash", "-ceu"]

# -------------------------
# Default
# -------------------------

default:
    just help

help:
    @just --list

# -------------------------
# Setup / Tooling
# -------------------------

setup-embedded:
    rustup target add thumbv7em-none-eabihf

# -------------------------
# Formatting
# -------------------------

fmt:
    cargo fmt --all

fmt-toml:
    taplo fmt

fmt-all: fmt fmt-toml

fmt-check:
    cargo fmt --all -- --check
    taplo fmt --check

# -------------------------
# Checks (host)
# -------------------------

check:
    cargo check --all-targets

check-std:
    cargo check --lib --features std

# -------------------------
# Embedded (no_std)
# -------------------------

check-no-std: setup-embedded
    cargo check --lib --no-default-features --target thumbv7em-none-eabihf

check-no-std-defmt: setup-embedded
    cargo check --lib --no-default-features --features defmt --target thumbv7em-none-eabihf

# -------------------------
# Linting
# -------------------------

lint:
    cargo clippy --all-targets --all-features -- -D warnings

lint-no-std: setup-embedded
    cargo clippy --lib --no-default-features --features defmt --target thumbv7em-none-eabihf -- -D warnings

# -------------------------
# Documentation
# -------------------------

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# -------------------------
# MSRV validation
# -------------------------

msrv:
    cargo +1.75.0 check --lib --no-default-features
    cargo +1.75.0 check --lib --features std
    cargo +1.75.0 check --lib --no-default-features --features defmt

# -------------------------
# Advanced checks
# -------------------------

hack:
    cargo hack check --feature-powerset --no-dev-deps

# -------------------------
# Tests
# -------------------------
#
# Run the full test suite (unit + integration + determenistic property tests).
# proptest-based tests in prop_test.rs are compiled automatically on host

# because std is always available in the cargo test harness.
test-host:
    cargo test

# Run only the deterministic property-based tests (no proptest, no std feature required — always works on any host target).
test-deterministic:
    cargo test --test prop_deterministic

# Run proptest-based property tests explicitly with the std feature.
# This is equivalent to `cargo test` on a host, but is explicit for CI jobs

# that want to isolate the proptest run.
test-props:
    cargo test --features std --test prop_tests

# Run all tests: unit, integration, deterministic properties, proptest.
test-all: test-host test-deterministic test-props

# no_std smoke-test (cannot actually run tests on bare-metal,

# but we verify the lib compiles for that target).
test-no-std: setup-embedded
    cargo check --lib --no-default-features --target thumbv7em-none-eabihf

# -------------------------
# CI aggregate
# -------------------------

ci: fmt-check lint check check-std check-no-std check-no-std-defmt msrv doc hack test-all

# -------------------------
# Cleanup
# -------------------------

clean:
    cargo clean
