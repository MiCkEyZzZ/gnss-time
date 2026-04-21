# The gnss-time Dec Commands

# Форматирование Rust-кода
fmt:
    cargo fmt --all

# форматирование всех Cargo.toml через Taplo
fmt-toml:
    taplo fmt

# Форматирование всего проекта (Rust + TOML)
fmt-all: fmt fmt-toml

# Проверка форматирования без изменения файлов (CI-safe)
fmt-check:
    cargo fmt --all -- --check
    taplo fmt --check

# Clippy: все таргеты и фичи, warnings -> errors
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Проверка no_std под embedded target
no-std:
    rustup target add thumbv7em-none-eabihf
    cargo check --target thumbv7em-none-eabihf --no-default-features --features alloc --lib

# Документация
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# MSRV check
msrv:
    cargo +1.75.0 check --all-features

# Очистка
clean:
    cargo clean
