# Gnss Time structure

```text
gnss-time
├── .cargo
│   └── config.toml
├── .config
│   └── nextest.toml
├── .github
│   ├── DISCUSSION_TEMPLATE
│   │   └── feature-requests.yml
│   ├── ISSUE_TEMPLATE
│   │   ├── bug_report.yml
│   │   ├── config.yml
│   │   └── enhancement.yml
│   ├── workflows
│   │   ├── ci.yml
│   │   ├── embedded.yml
│   │   ├── fuzz.yml
│   │   ├── msrv.yml
│   │   ├── publish.yml
│   │   ├── release.yml
│   │   └── semantic-pull-request.yml
│   ├── CODEOWNERS
│   ├── dependabot.yml
│   ├── FUNDING.yml
│   └── pull_request_template.md
├── benches
│   ├── benches
│   │   ├── arithmetic_bench.rs
│   │   ├── convert_bench.rs
│   │   ├── providers_bench.rs
│   │   ├── time_bench.rs
│   │   └── vs_hifitime.rs
│   ├── target
│   ├── Cargo.lock
│   ├── Cargo.toml
│   └── README.md
├── docs
│   ├── API_STABILITY.de.md
│   ├── API_STABILITY.md
│   ├── API_STABILITY.ru.md
│   ├── ARCHITECTURE.de.md
│   ├── ARCHITECTURE.md
│   ├── ARCHITECTURE.ru.md
│   ├── EMBEDDED.de.md
│   ├── EMBEDDED.md
│   ├── EMBEDDED.ru.md
│   ├── GNSS_TIME_PRIMER.de.md
│   ├── GNSS_TIME_PRIMER.md
│   ├── GNSS_TIME_PRIMER.ru.md
│   ├── INVARIANTS.de.md
│   ├── INVARIANTS.md
│   ├── INVARIANTS.ru.md
│   ├── LEAP_SECONDS.de.md
│   ├── LEAP_SECONDS.md
│   ├── LEAP_SECONDS.ru.md
│   └── PROJECT_STRUCTURE.md
├── examples
│   ├── basic_usage.rs
│   ├── chain_conversion.rs
│   ├── civil_time.rs
│   ├── convert_basic.rs
│   ├── convert_contextual.rs
│   ├── display_formats.rs
│   ├── dynamic_conversion.rs
│   ├── embedded_minimal
│   ├── embedded_safe_arithmetic.rs
│   ├── glonass_day_tod.rs
│   ├── glonass_receiver.rs
│   ├── gps_time_operations.rs
│   ├── gps_week_tow.rs
│   ├── log_stream.rs
│   ├── matrix_inspection.rs
│   ├── multi_constellation.rs
│   ├── no_domain_mixing.rs
│   ├── parse_time.rs
│   ├── README.md
│   ├── receiver_timestamp.rs
│   ├── scale_conversion.rs
│   ├── sync_alignment.rs
│   └── unix_time.rs
├── firmware
│   ├── src
│   │   └── main.rs
│   ├── Cargo.lock
│   ├── Cargo.toml
│   ├── memory.x
│   └── README.md
├── fuzz
│   ├── artifacts
│   ├── corpus
│   ├── fuzz_targets
│   │   ├── fuzz_day_tod.rs
│   │   ├── fuzz_gps_utc.rs
│   │   ├── fuzz_leap_lookup.rs
│   │   ├── fuzz_try_extend.rs
│   │   ├── fuzz_utc_to_gps.rs
│   │   └── fuzz_week_tow.rs
│   ├── .gitignore
│   ├── Cargo.lock
│   ├── Cargo.toml
│   └── README.md
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
├── .gitattributes
├── .gitignore
├── AUTHOR.md
├── Cargo.lock
├── Cargo.toml
├── CHANGELOG.md
├── clippy.toml
├── codecov.yml
├── deny.toml
├── INSTALL
├── justfile
├── LICENSE.APACHE
├── LICENSE.MIT
├── README.md
├── release-plz.toml
├── rust-toolchain.toml
├── rustfmt.toml
├── taplo.toml
└── tombi.toml
```
