# The gnss-time Dec Commands

set shell := ["bash", "-ceu"]

default:
    just help

help:
    @just --list

setup-embedded:
    rustup target add thumbv7em-none-eabihf

fmt:
    cargo fmt --all

fmt-toml:
    taplo fmt

fmt-all: fmt fmt-toml

fmt-check:
    cargo fmt --all -- --check
    taplo fmt --check

check:
    cargo check --all-targets

check-std:
    cargo check --lib --features std

check-no-std: setup-embedded
    cargo check --lib --no-default-features --target thumbv7em-none-eabihf

check-no-std-defmt: setup-embedded
    cargo check --lib --no-default-features --features defmt --target thumbv7em-none-eabihf

lint:
    cargo clippy --all-targets --all-features -- -D warnings

lint-no-std: setup-embedded
    cargo clippy --lib --no-default-features --features defmt --target thumbv7em-none-eabihf -- -D warnings

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

msrv:
    cargo +1.75.0 check --lib --no-default-features
    cargo +1.75.0 check --lib --features std
    cargo +1.75.0 check --lib --no-default-features --features defmt

hack:
    cargo hack check --feature-powerset --no-dev-deps

test-host:
    cargo test

test-no-std: setup-embedded
    cargo check --lib --no-default-features --target thumbv7em-none-eabihf

ci: fmt-check lint check check-std check-no-std check-no-std-defmt msrv doc hack

clean:
    cargo clean
