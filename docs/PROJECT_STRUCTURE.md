# Project structure

```text
gnss-time
├── benches
│   └── time_bench.rs
├── docs
│   ├── duration.txt
│   ├── epoch.txt
│   ├── PROJECT_STRUCTURE.md
│   ├── README.txt
│   ├── ROADMAP.md
│   ├── scale.txt
│   └── time.txt
├── examples
│   ├── basic_usage.rs
│   ├── chain_conversion.rs
│   ├── convert_basic.rs
│   ├── convert_contextual.rs
│   ├── display_formats.rs
│   ├── embedded_safe_arithmetic.rs
│   ├── glonass_day_tod.rs
│   ├── glonass_receiver.rs
│   ├── gps_week_tow.rs
│   ├── log_stream.rs
│   ├── matrix_inspection.rs
│   ├── multi_constellation.rs
│   ├── no_domain_mixing.rs
│   ├── README.md
│   ├── receiver_timestamp.rs
│   ├── scale_conversion.rs
│   └── sync_alignment.rs
├── src
│   ├── tables
│   │   ├── leap_seconds.rs
│   │   └── mod.rs
│   ├── convert.rs
│   ├── duration.rs
│   ├── epoch.rs
│   ├── error.rs
│   ├── leap.rs
│   ├── lib.rs
│   ├── matrix.rs
│   ├── prelude.rs
│   ├── scale.rs
│   └── time.rs
├── tests
│   ├── glonass_test.rs
│   ├── roundtrip_test.rs
│   └── time_integration_test.rs
├── .editorconfig
├── .gitignore
├── AUTHOR.md
├── Cargo.lock
├── Cargo.toml
├── CHANGELOG.md
├── clippy.toml
├── INSTALL
├── justfile
├── LICENSE.APACHE
├── LICENSE.MIT
├── README.md
├── rust-toolchain.toml
├── rustfmt.toml
└── taplo.toml
```
