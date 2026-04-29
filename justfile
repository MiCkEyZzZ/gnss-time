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

test-host:
    cargo test

test-no-std: setup-embedded
    cargo check --lib --no-default-features --target thumbv7em-none-eabihf

# -------------------------
# CI aggregate
# -------------------------

ci: fmt-check lint check check-std check-no-std check-no-std-defmt msrv doc hack

# -------------------------
# Cleanup
# -------------------------

clean:
    cargo clean
