# Fuzzing testing - `gnss-time`

This directory contains [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html)
targets for `gnss-time`. All targets use `libFuzzer` via the `libfuzzer-sys` crate.

## Prerequisites

```sh
rustup toolchain install nightly
rustup target add x86_64-unknown-linux-gnu  # libFuzzer runs on Linux
cargo install cargo-fuzz --locked
```
