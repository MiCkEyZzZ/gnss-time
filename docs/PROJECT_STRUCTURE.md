# Gnss Time structure

```text
gnss-time
├── .cargo
│   └── config.toml
├── .github
│   ├── ISSUE_TEMPLATE
│   │   ├── bug_report.yml
│   │   ├── config.yml
│   │   └── enhancement.yml
│   ├── workflows
│   │   ├── ci.yml
│   │   ├── embedded.yml
│   │   ├── publish.yml
│   │   └── semantic-pull-request.yml
│   ├── CODEOWNERS
│   ├── dependabot.yml
│   ├── FUNDING.yml
│   └── pull_request_template.md
├── benches
│   ├── benches
│   │   ├── arithmetic_bench.rs
│   │   ├── convert_bench.rs
│   │   └── time_bench.rs
│   ├── target
│   ├── Cargo.lock
│   ├── Cargo.toml
│   └── README.md
├── docs
│   ├── ARCHITECTURE.md
│   ├── duration.txt
│   ├── EMBEDDED.md
│   ├── epoch.txt
│   ├── GNSS_TIME_PRIMER.md
│   ├── INVARIANTS.md
│   ├── leap.txt
│   ├── LEAP_SECONDS.md
│   ├── PROJECT_STRUCTURE.md
│   ├── README.txt
│   ├── ROADMAP.md
│   ├── ROADMAP_2.md
│   ├── scale.txt
│   └── time.txt
├── examples
│   ├── basic_usage.rs
│   ├── chain_conversion.rs
│   ├── civil_time.rs
│   ├── convert_basic.rs
│   ├── convert_contextual.rs
│   ├── display_formats.rs
│   ├── dynamic_conversion.rs
│   ├── embedded_safe_arithmetic.rs
│   ├── glonass_day_tod.rs
│   ├── glonass_receiver.rs
│   ├── gps_time_operations.rs
│   ├── gps_week_tow.rs
│   ├── log_stream.rs
│   ├── matrix_inspection.rs
│   ├── multi_constellation.rs
│   ├── no_domain_mixing.rs
│   ├── README.md
│   ├── receiver_timestamp.rs
│   ├── scale_conversion.rs
│   ├── sync_alignment.rs
│   └── unix_time.rs
├── src
│   ├── tables
│   │   ├── leap_seconds.rs
│   │   └── mod.rs
│   ├── civil.rs
│   ├── convert.rs
│   ├── duration.rs
│   ├── epoch.rs
│   ├── error.rs
│   ├── leap.rs
│   ├── lib.rs
│   ├── matrix.rs
│   ├── prelude.rs
│   ├── scale.rs
│   ├── serde_impls.rs
│   └── time.rs
├── tests
│   ├── glonass_test.rs
│   ├── no_std_compact.rs
│   ├── prop_deterministic.rs
│   ├── prop_tests.rs
│   ├── roundtrip_test.rs
│   ├── serde_test.rs
│   └── time_integration_test.rs
├── .editorconfig
├── .gitignore
├── AUTHOR.md
├── Cargo.lock
├── Cargo.toml
├── CHANGELOG.md
├── clippy.toml
├── deny.toml
├── INSTALL
├── justfile
├── LICENSE.APACHE
├── LICENSE.MIT
├── README.md
├── rust-toolchain.toml
├── rustfmt.toml
└── taplo.toml
```
