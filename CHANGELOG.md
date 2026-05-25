# Changelog

All notable changes to **gnss-time** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Introduce named constants for the Hinnant civil date conversion
  algorithm in `civil.rs` to remove remaining magic numbers and
  improve symmetry with `civil_from_days`.
  - Add DAYS_PER_ERA, DAYS_PER_4_YEAR_CYCLE, DAYS_PER_100_YEAR_CYCLE
  - Add UNIX_EPOCH_FROM_CIVIL constant for epoch offset clarity
  - Improve readability and maintainability of `days_to_unix`
  - Align structure with `civil_from_days` implementation style

- Introduce named constants for the Hinnant civil date conversion
  algorithm in `epoch.rs` to remove remaining magic numbers and
  improve symmetry with `days_from_unix_impl`.
  - Add DAYS_PER_400_YEAR_ERAL, DAYS_FROM_CIVIL_TO_UNIX_EPOCH, YARS_PER_ERA
  - Improve readability and maintainability of `days_from_unix_impl`

- Introduced `CivilDateTime` — a proleptic Gregorian calendar date-time
  representation derived from `Time<Utc>`.

- Added conversion API:
  - `Time<Utc>::to_civil() -> CivilDateTime`
  - `CivilDateTime::from_utc_nanos(nanos: u64) -> Result<CivilDateTime, GnssTimeError>`
  - `CivilDateTime::to_utc() -> Result<Time<Utc>, GnssTimeError>`
  - `CivilDateTime::to_utc_nanos() -> Result<u64, GnssTimeError>`

- Implemented full `CivilDateTime` structure with nanosecond precision:
  - `year`, `month`, `day`
  - `hour`, `minute`, `second`
  - `nanos` (sub-second component, 0–999_999_999)

- Added `Display` implementation for ISO 8601 / RFC 3339 formatting:
  - Format: `YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ`
  - Example: `2024-01-15T12:34:56.123456789Z`

- Ensured lossless round-trip conversions:
  - `Time<Utc> → CivilDateTime → Time<Utc>` preserves exact nanoseconds

- Added comprehensive test coverage for:
  - Epoch boundary (`1972-01-01T00:00:00Z`)
  - GPS epoch alignment
  - Leap year correctness
  - Sub-second precision handling
  - Day boundary transitions
  - Round-trip correctness across full range

- Added `is_whole_second()` helper for fast sub-second checks

- Added example:
  - `examples/civil_time.rs` demonstrating formatting and conversion usage

- Enforced strict `no_std` compatibility for core conversion logic
  (formatting remains conditionally available depending on feature flags)

- `Display` implementation is feature-gated depending on formatting strategy
  (`std` / `alloc`)

- Added comprehensive Postcard-based serialization test suite (`tests/serde_test.rs`,
  33 tests, behind `serde` feature):
  - Full round-trip coverage for `Time<S>` across all scales (`Gps`, `Utc`, `Tai`,
    `Galileo`, `Beidou`, `Glonass`), including:
    - `EPOCH`, `MAX`, and sub-second precision values

  - Wire format validation aligned with Postcard ULEB-128 encoding:
    - `0` → 1 byte (`[0x00]`)
    - `u64::MAX` → 10 bytes
    - Any `Time<S>` → ≤ 10 bytes
    - 1-week timestamp → 8 bytes

  - Raw byte-level encoding tests:
    - `1 ns` → `[0x01]`
    - `127 ns` → `[0x7F]`
    - `128 ns` → `[0x80, 0x01]`

  - Verified scale isolation:
    - Identical nanoseconds produce identical wire format across scales
    - Correct type-safe deserialization per scale

  - Added round-trip tests for:
    - `Duration` (`ZERO`, positive, negative, `MIN`, `MAX`)
    - `DurationParts` (including boundary values)

  - Verified compatibility with heapless environments:
    - Confirmed that a 16-byte buffer is sufficient for all supported types

  - Cross-format consistency tests:
    - JSON ↔ Postcard round-trip equivalence
    - Macro-based validation across all time scales

  - Integration tests:
    - GPS ↔ UTC conversions with leap seconds + Postcard round-trip
    - Unix time ↔ UTC + Postcard round-trip

- Added `heapless = "0.8"` to `dev-dependencies` for embedded serialization testing

- Added `[[test]]` configuration:
  - `serde_test` is compiled only when `--features serde` is enabled

- Extended `docs/EMBEDDED.md` with **Compact binary serialization (Postcard)** section:
  - Formal wire format specification:
    - `Time<S>` → ULEB-128 (`u64`, 1–10 bytes)
    - `Duration` → ZigZag + ULEB-128 (`i64`)
    - `DurationParts` → tuple `[u64, u32]`

  - Clarified that Postcard encoding is variable-length (not fixed 8 bytes)
  - Added recommended buffer sizing guidelines (≥ 16 bytes)
  - Added `heapless::Vec` example for `no_std` environments
  - Included telemetry packet example with real encoded size estimation
  - Documented `is_human_readable()` behavior for JSON vs binary formats

- Added optional `serde` support behind the `serde` feature flag.
  - New module `serde_impls.rs` (compiled only with `#[cfg(feature = "serde")]`).

- Added custom Serde implementations for core time types:
  - `Time<S>` — dual-format serialization:
    - Human-readable: `{ "scale": "GPS", "nanos": 1356566418000000000 }`
      - Includes runtime validation of `scale` field against the compile-time
        time scale.
      - Deserialization returns a descriptive error on mismatch.

    - Compact: raw `u64` nanoseconds (no scale tag, type-level encoding guarantees
      correctness).

  - `Duration`:
    - Human-readable: `{ "nanos": -7000000000 }`
    - Compact: raw `i64` nanoseconds

  - `DurationParts`:
    - Human-readable: `{ "seconds": 5, "nanos": 500000000 }`
    - Compact: `[seconds, nanos]` tuple encoding
    - Enforces invariant `nanos < 1_000_000_000` via constructor validation

- Implemented Serde using manual visitor-based deserialization (no proc-macros).
  - Uses `is_human_readable()` to switch formats at runtime.
  - Fully `no_std` compatible (when `serde` is built with `default-features = false`).
  - Error reporting implemented without allocations using `fmt::Display`.

- Added 42 Serde-related tests:
  - Exact JSON format validation
  - Round-trip tests for all supported time scales
  - Error cases for scale mismatches
  - Compact vs human-readable consistency checks
  - Postcard binary round-trip tests
  - Boundary tests (`EPOCH`, `MIN`, `MAX`)
  - Size comparison tests (compact vs JSON)
  - Integration tests across multiple time scales

- Added documentation section: **Serde support**
  - Describes all formats per type
  - Includes usage examples
  - Explains design constraints (no proc-macro, no alloc)

- Extended `docs/ARCHITECTURE.md`:
  - Added Serde design rationale
  - Added serialization format specification table
  - Updated feature flag matrix
  - Updated CI guarantees section

- Updated `Cargo.toml`:
  - Added optional dependency:

    ```toml
    serde = { version = "1", default-features = false, optional = true }
    ```

  - Added feature flag:

    ```toml
    serde = ["dep:serde"]
    ```

  - Added `alloc` feature flag (preparation for future extensions)
  - Added dev-dependencies:
    - `serde_json`
    - `postcard` (with `alloc` feature for tests)

  - Updated `docs.rs` metadata to include `serde` feature

- Updated `src/lib.rs`:
  - Added conditional module:

    ```rust
    #[cfg(feature = "serde")]
    mod serde_impls;
    ```

  - Added Serde usage example in crate-level documentation
  - Updated feature flags documentation table

- в `Cargo.toml` добавлен serde, как опциональная зависимость

- Added Unix/UTC/GPS epoch constants:
  - `UTC_EPOCH_UNIX_OFFSET_S`
  - `UTC_EPOCH_UNIX_OFFSET_NS`
  - `GPS_EPOCH_UNIX_S`
  - `UTC_CIVIL_EPOCH`

- Added Unix time conversion API:
  - `Time<Utc>::from_unix_seconds`
  - `Time<Utc>::from_unix_nanos`
  - `Time<Utc>::as_unix_seconds`
  - `Time<Utc>::as_unix_nanos`
  - `Time<Gps>::from_unix_seconds` (with `LeapSecondsProvider`)
  - `Time<Gps>::as_unix_seconds` (with `LeapSecondsProvider`)

- Enforced UTC epoch lower bound (1972-01-01) for Unix → UTC conversions

- Added unit tests covering Unix ↔ UTC ↔ GPS round-trips and edge cases

- Added `unix_time.rs` example with 8 sections and complete demonstrations:
  - Epoch constants — demonstration of constants
  - Unix epoch before UTC epoch → error case
  - UTC epoch from Unix → UTC epoch
  - Round-trip seconds for 8 historical dates
  - Round-trip nanoseconds with sub-second precision
  - GPS ↔ Unix via UTC + leap seconds
  - Verification that GPS−UTC = 18 in 2023
  - Integration pattern with `std::time::SystemTime`

- Created a new `bench` crate and moved existing benchmark tests into it:
  - `arithmetic_bench.rs`
  - `convert_bench.rs`
  - `time_bench.rs`

### Changed

- Updated `Cargo.toml`:
  - Extended documentation for `postcard` dependency:
    - Clarified `alloc` vs heapless usage modes

  - Improved comments around feature-gated serialization support

- Improved architecture documentation consistency between:
  - `ARCHITECTURE.md`
  - crate-level docs
  - feature flag definitions in `Cargo.toml`

- Clarified serialization strategy:
  - Human-readable format is schema-based (not string-based)
  - Compact format is strictly zero-overhead (no tags, no allocation)

- Added constants to `prelude.rs`:
  - `GPS_EPOCH_UNIX_S`
  - `UTC_EPOCH_UNIX_OFFSET_NS`
  - `UTC_EPOCH_UNIX_OFFSET_S`

- Updated `prelude.rs` with re-exports:
  - `UTC_EPOCH_UNIX_OFFSET_S`
  - `UTC_EPOCH_UNIX_OFFSET_NS`
  - `GPS_EPOCH_UNIX_S` (now available via `use`)

- Improved code documentation in `duration.rs`

- Updated `README.md` with minimum required Rust version

- Added links to license files

- Added constants to `prelude.rs`

### Fixed

- **`Time<Utc>::as_unix_nanos`**: previously a `u64` → `i64` cast could wrap to
  negative values for timestamps exceeding `i64::MAX`. The method now uses
  `i64::try_from` and saturates at `i64::MAX`, matching its documented behaviour.

### Removed

- Removed the `benches` directory

## [0.5.2] - 2026-05-04

### Added

- updated the examples in the README file, improved the code documentation

### Changed

- updated the examples in the README file, improved the code documentation

## [0.5.1] - 2026-05-03

### Added

- Updated README file

## [0.5.0] - 2026-05-03

### Added

- Deterministic property tests in `tests/prop_deterministic.rs`:
  - fixed sample coverage for boundary values, all 18 leap-second transitions,
    uniform `u64` range coverage, and real IGS epochs
  - deterministic invariants for GPS→TAI→GPS, GPS→Galileo→GPS, GPS→BeiDou→GPS,
    GPS→UTC→GPS, arithmetic laws, monotonicity, ambiguity windows, and sub-second
    edge cases
- Randomized property tests in `tests/prop_tests.rs` using `proptest`:
  - GPS domain sampling across the supported range
  - bounded duration strategies to avoid arithmetic overflow in law checks
  - leap-second boundary sampling within ±3 seconds
  - dedicated ambiguity coverage to ensure `ConvertResult::Exact` outside leap windows
- `justfile` test recipes:
  - `test-deterministic`
  - `test-props`
  - `test-all`

- Compile-time verification for leap second table (`BUILTIN_TABLE`):
  - `_ASSERT_FIRST_ENTRY` — validates initial offset (TAI−UTC = 19)
  - `_ASSERT_TABLE_INVARIANTS` — enforces strict ordering and +1 increments
  - `_ASSERT_LAST_ENTRY` — validates last entry (2017-01-01, TAI−UTC = 37)
- `LeapSeconds::last_update() -> Option<Time<Tai>>` — returns last leap second
  event (TAI)
- `LeapSeconds::current_tai_minus_utc() -> i32` — accessor for current offset
- `RuntimeLeapSeconds`:
  - fixed-capacity, heap-free runtime leap second table (`RUNTIME_CAPACITY = 64`)
  - `from_builtin()` — initialize from compile-time snapshot
  - `from_slice()` — construct from external data
  - `try_extend()` — validated append API
- `LeapExtendError` (`#[non_exhaustive]`) with variants:
  - `NotStrictlyAscending`
  - `NonUnitIncrement`
  - `BufferFull`
- Prelude exports:
  - `RuntimeLeapSeconds`, `LeapExtendError`, `LeapEntry`, `RUNTIME_CAPACITY`
  - helper functions `gps_to_utc`, `utc_to_gps`
- Test `test_builtin_table_matches_iers_bulletin_c`:
  - full cross-verification against IERS data

- Added `#[must_use]` annotations across core types and APIs:
  - `Time<S>`, `Duration`, `GnssTimeError`, `ConvertResult<T>`
  - all arithmetic helpers (`checked_*`, `saturating_*`)
  - accessors (`as_nanos`, `as_seconds`, `week`, `tow_seconds`, etc.)
  - conversion traits (`IntoScale`, `IntoScaleWith`)
- Added diagnostic messages to `#[must_use]` where ignoring results is likely a
  bug
- Added `#[non_exhaustive]` to:
  - `GnssTimeError`
  - `ConvertResult<T>`
  - `ConversionKind`
  - `ScaleId`
    (allows future extension without breaking changes)
- Added helper methods in `scale.rs`:
  - `OffsetToTai::is_fixed()`
  - (complements existing `is_contextual()`)
- Added extended trait coverage test:
  - `test_scale_is_copy` now validates `Copy + Clone + Eq + PartialEq + Debug`
- Added test `test_offset_to_tai_helpers` covering `is_fixed()` and `is_contextual()`
- Added `clippy.toml` as an explicit lint configuration entry point
  for future lint expansion

### Changed

- Split property-based testing into deterministic and `proptest`-based suites to
  keep `no_std` compatibility clean while still providing host-side randomized coverage
- Updated `proptest` dependency configuration in `Cargo.toml` to explicitly
  require `std`:
  - `default-features = false`
  - `features = ["std"]`
- Added `test-all` to the CI-oriented `just` workflow

- Leap second table (`BUILTIN_TABLE`) fully verified against IERS Bulletin C
  (all 19 entries validated using threshold formula)
- Updated leap second documentation:
  - added update policy and maintenance workflow
  - documented compile-time invariants
  - added runtime extension examples and validation rules
  - included IERS monitoring references and current status (TAI−UTC = 37 as of 2026)
- `LeapSeconds` API symmetry improved:
  - `from_slice` added as alias to `from_table` for clarity
- Enabled crate-wide lint:
  - `#![warn(clippy::must_use_candidate)]`

- Improved API correctness by enforcing explicit result usage via `#[must_use]`
- Strengthened forward-compatibility guarantees using `#[non_exhaustive]`
- Updated internal documentation and diagnostics for better developer feedback
- Prepared clippy configuration for future lint additions
  (`warn-on-all-wildcard-imports = false` as extension point)

### Fixed

- Fixed documentation typo in `GnssTimeError` (`i64` → `u64` for internal representation)

## [0.4.0] — 2026-05-02

### Added

- Full CI/CD pipeline:
  - `ci.yml` — formatting, clippy (all feature sets), tests, docs, MSRV, cargo-deny
  - `embedded.yml` — cross-compilation for embedded targets (Cortex-M, RISC-V)
    with `no_std` validation
  - reusable workflow integration (`workflow_call`)
- `publish.yml` — automated crates.io release pipeline:
  - waits for CI to succeed on tagged commit
  - verifies tag ↔ Cargo.toml version consistency
  - `cargo publish --dry-run` (preflight validation)
  - `cargo publish` with protected environment
  - automatic GitHub Release generation
- `docs/ARCHITECTURE.md` — internal design, module layout, TAI pivot, feature flags
- `docs/EMBEDDED.md` — embedded usage guide with UBX/GLONASS parsing examples,
  benchmark table
- `docs/GNSS_TIME_PRIMER.md` — GPS/GLONASS/UTC/TAI explained for developers
- `docs/INVARIANTS.md` — type-level, arithmetic, conversion and memory invariants
- `docs/LEAP_SECONDS.md` — full leap second table reference with source citations
- `examples/README.md` — examples index with benchmark results
- Property-based tests `tests/prop_tests.rs` (9 tests):
  - Roundtrip GPS→UTC→GPS (256 sample points, all leap second boundaries, real
    IGS epochs)
  - Roundtrip GPS→GAL→GPS, GPS→BDT→GPS, GPS→TAI→GPS
  - Sort order `Vec<Time<Gps>>` matches internal `u64` order
  - GPS→UTC monotonicity between leap second events
  - GPS−UTC offset verification at known epochs
  - All 18 historical leap second transitions (1981–2017)
  - Strict GPS−UTC offset increase at each transition
- `Time::NANOS_PER_YEAR` constant (365 × 24 × 3600 × 10⁹ ns)
- Overflow boundary tests in `src/time.rs`:
  `checked_add` / `checked_sub` near `u64::MAX` and `EPOCH`,
  `saturating_add` / `saturating_sub_duration`,
  `try_add` / `try_sub_duration`,
  panicking operators panic on overflow,
  `checked_elapsed` near `i64` boundary

### Changed

**API breaking change**: replaced `f64` fractional seconds with `DurationParts`
for all constructors:

- `Time<Gps>::from_week_tow(week, tow)` now takes `DurationParts`

- `Time<Glonass>::from_day_tod(day, tod)` now takes `DurationParts`

- Added construction-time validation: `seconds` and `nanos` are checked
  against valid ranges

- Eliminated non-determinism related to `f64`

- **New type `DurationParts`**:
  - Fields `seconds: u64` and `nanos: u32`
  - `new()` constructor with validation `nanos < 1_000_000_000`
  - Method `as_nanos() -> u128` for conversion to nanoseconds

- **Updated all examples** (`examples/`):
  - `basic_usage.rs`, `gps_week_tow.rs`, `glonass_day_tod.rs`
  - `convert_basic.rs`, `convert_contextual.rs`, `chain_conversion.rs`
  - `display_formats.rs`, `dynamic_conversion.rs`, `embedded_safe_arithmetic.rs`
  - `glonass_receiver.rs`, `gps_time_operations.rs`, `log_stream.rs`
  - `matrix_inspection.rs`, `multi_constellation.rs`, `no_domain_mixing.rs`
  - `no_std_example.rs`, `receiver_timestamp.rs`, `scale_conversion.rs`
  - `sync_alignment.rs`

- **Updated integration tests** (`tests/`):
  - `glonass_test.rs` — all constructors rewritten to use `DurationParts`
  - `roundtrip_test.rs` — all roundtrip tests updated
  - `time_integration_test.rs` — adapted to the new API

- **Updated benchmarks** (`benches/`):
  - `arithmetic_bench.rs` — unchanged (does not use constructors)
  - `convert_bench.rs` — updated `from_week_tow` calls to use `DurationParts`
  - `time_bench.rs` — constructors updated

- **Documentation**:
  - Full documentation added for `DurationParts`
  - Updated examples in doc comments across all modules

- CI architecture:
  - embedded checks extracted into reusable workflow (`embedded.yml`)
  - improved caching strategy (feature-aware cache keys)
  - stricter validation (`-D warnings`, clippy across all feature combinations)

- `benches/arithmetic_bench.rs`: added `checked_add`, `checked_sub_duration`,
  `saturating_add`, `Duration` benchmarks; updated target figures

- `benches/convert_bench.rs`: added `leap_second_lookup` microbenchmark

## [0.3.0] — 2026-04-27

### Added

- **Benchmarks (#TIME-12)**: added `benches/arithmetic_bench.rs` and `benches/convert_bench.rs`.
  - Demonstrate zero-cost abstractions: `Time<Gps> + Duration` (512 ps) on par with
    `u64 + u64` (517 ps).
  - Conversions without leap seconds: ~0.8–0.9 ns.
  - `GPS → UTC` conversion with leap seconds: ~9.5 ns (under 10 ns).
  - Uses `criterion` with HTML reports.

- **time.rs**: added `Time::MIN` constant (alias of `EPOCH`) for symmetry with `MAX`.

- **time.rs**: added documentation describing the value range of `Time<S>` (~584
  years from the epoch; for GPS up to year 2554).

- **time.rs**: added `test_time_max_behavior` to verify behavior near `u64::MAX`.

- **.github/workflows/embedded.yml**: added `clippy::arithmetic_overflow` check
  to the lint job.

- Added **Issue template `enhancement.yml`** for proposing improvements to existing
  functionality.
  - Categories: performance, API, time scale conversions, leap seconds,
    embedded/no_std, formatting, refactoring, testing, documentation.

- Added **`CODEOWNERS` file** for automatic ownership assignment across repository
  areas.
  - Defines responsibility for source (`/src/`), tests (`/tests/`), benchmarks
    (`/benches/`), examples (`/examples/`), CI/CD (`/.github/workflows/`), documentation
    (`/docs/`), and root files.
  - Used by GitHub for automatic reviewer assignment on Pull Requests.

- Added **Pull Request template** (`.github/pull_request_template.md`).
  - Provides a structured checklist for reviewing changes: scope, description,
    and testing approach.
  - Includes required checks: `cargo fmt`, `taplo format`, `cargo clippy`,
    `cargo test`, documentation, and CHANGELOG updates.

- **CI**: added GitHub Actions workflow to validate semantic Pull Request titles
  (`.github/workflows/semantic-pull-request.yml`).
  - Automatically enforces PR title format: `type(scope?): description`.
  - Supported types: `feat`, `fix`, `docs`, `chore`, `perf`, `refactor`,
    `test`, `ci`, `build`, `style`.
  - Runs only on non-draft PRs (drafts are ignored).
  - Leaves an automatic comment if the title is invalid.

- **.github/workflows**
  - added `embedded.yml` for type size checks, builds for
    `thumbv7em-none-eabihf`, `thumbv7em-none-eabi`, `riscv32imac-unknown-none-elf`,
    host tests, and clippy.

- **.cargo**
  - added `config.toml` for cross-compilation:
    - `thumbv7em-none-eabihf`
    - `thumbv7em-none-eabi`
    - `thumbv6m-none-eabi`
    - `riscv32imac-unknown-none-elf`
    - `riscv32i-unknown-none-elf`
    - `opt-level = "s"` — minimize binary size for flash-constrained devices
    - `codegen-units = 1` — improved optimization
    - `-C link-arg=-Tlink.x` for Cortex-M (requires linker script from `cortex-m-rt`)
    - `-D warnings` — treat warnings as errors in embedded CI

- **tests**
  - added `no_std_compact.rs` tests verifying:
    - absence of `Drop`
    - `Copy` semantics
    - `const fn` usability in static context
    - 8-byte alignment for DMA
    - no allocations in conversion paths
    - `core::fmt` without `std`
    - enforcement of `#![forbid(unsafe_code)]`

- **time.rs**
  - added `impl<S: TimeScale> defmt::Format for Time<S>` under
    `#[cfg(feature = "defmt")]`

- **error.rs**
  - added `impl defmt::Format for GnssTimeError` under `#[cfg(feature = "defmt")]`

### Changed

- `Cargo.toml`: bumped to `0.3.0`; added `defmt = ["dep:defmt"]` using
  `dep:` syntax (Cargo 1.60+); added `[package.metadata.docs.rs]` for
  docs.rs targets and features.

- `justfile`: added commands:
  `setup-embedded`, `check-std`, `check-no-std`,
  `check-no-std-defmt`, `lint-no-std`, `msrv`, `hack`,
  `test-host`, `test-no-std`, `ci`.

### Fixed

- `leap.rs`: `LeapSeconds::builtin()` now returns `&'static LeapSeconds`
  (previously `const fn`, which is incompatible with `no_std` static data access).

- `time.rs`: removed `const` from `as_seconds_f64` (floating-point operations
  are not `const` on stable Rust 1.75).

## [0.2.0] — 2026-04-26

### Added

- **Full conversion matrix (`matrix`)**:
  - `ScaleId` type for runtime identification of time scales (GPS, GLONASS, Galileo,
    BeiDou, TAI, UTC).
  - `ConversionKind` type — classification of conversion types (Fixed, Identity,
    EpochShift, Contextual, SameScale).
  - `ConversionMatrix` structure — validation of compatibility and statistics over
    the conversion graph.
  - TAI offset constants: `TAI_OFFSET_GPS_NS`, `TAI_OFFSET_GALILEO_NS`, `TAI_OFFSET_BEIDOU_NS`,
    `TAI_OFFSET_TAI_NS`, `GLONASS_UTC_EPOCH_SHIFT_NS`.
  - `beidou_via_gps_to_glonass_via_utc` function — example of chained conversion
    across multiple scales.
  - Tests covering symmetry and classification of all 30 off-diagonal conversion
    paths.

- **Extended conversion capabilities in `leap` and `convert`**:
  - Functions: `galileo_to_utc`, `galileo_to_glonass`, `beidou_to_utc`, `beidou_to_glonass`,
    and reverse conversions `utc_to_galileo`, `utc_to_beidou`.
  - Implementations of `IntoScale` and `IntoScaleWith` for all scale pairs, including
    Galileo ↔ GLONASS, BeiDou ↔ GLONASS, Galileo ↔ UTC, BeiDou ↔ UTC.
  - Full support for a 6×6 conversion matrix (30 directional paths).

- Fixed a typo in `matrix.rs` doctest (`needs_leap_seconds` method and corrected
  number of contextual paths: 16 instead of 22).

- **New examples**:
  - `matrix_inspection.rs` — prints the conversion matrix.
  - `dynamic_conversion.rs` — runtime (dynamic) conversions.
  - `chain_conversion.rs` — end-to-end BeiDou → TAI conversion chain.

- **GLONASS-specific methods** (`Time<Glonass>`):
  - `sub_second_nanos()` — nanosecond fraction of the current second.
  - `day_of_week()` — ISO weekday (1 = Monday … 7 = Sunday), based on the 1996-01-01
    epoch (Monday).
  - `is_weekend()` — returns `true` for Saturday or Sunday.

- **GLONASS integration tests** (`tests/glonass_test.rs`):
  - Verification of constant GLO ↔ UTC offset (no leap seconds).
  - Roundtrip tests: GLO → UTC → GLO and GLO → GPS → GLO.
  - Validation of `day_of_week()` against known dates.
  - Behavior at the leap second boundary (2017-01-01).

- **Unified conversion API (`convert`)**:
  - `IntoScale<Target>` trait for fixed-offset conversions (GPS↔TAI, GPS↔Galileo,
    GPS↔BeiDou, GLO↔UTC).
  - `IntoScaleWith<Target>` trait for contextual conversions (GPS↔UTC, GPS↔GLO)
    with explicit `LeapSecondsProvider`.
  - `ConvertResult<T>` type for handling the ambiguous 1-second window during leap
    second insertion.
  - `into_scale_with_checked` method for detecting timestamps within a leap second.

- **`prelude` module** — convenient import of commonly used types:

  ```rust
  use gnss_time::prelude::*;
  ```

- **New examples (`examples/`)**:
  - `convert_basic.rs` — fixed-offset conversions (no leap seconds).
  - `convert_contextual.rs` — GPS <-> UTC conversions with leap seconds and ambiguity
    detection.

- **Integration tests (`tests/`)**:
  - `roundtrip_test.rs` — roundtrip accuracy across all scales, covering 18 leap
    second transitions and known RINEX epochs.
  - `time_integration_test.rs` — end-to-end usage scenarios.

### Fixed

- `utc_to_gps`: replaced a single-pass approximation with a two-pass algorithm.
  Roundtrip `GPS → UTC → GPS` is now exact (< 1 ns) across all 18 GPS-era leap
  second boundaries.

### Documentation

- Added documentation for the `convert` module, including a table of supported
  conversions and usage examples.
- Added `prelude` for more convenient imports.

## [0.1.0] — 2026-04-21

- **`Duration`** — signed nanosecond interval (`i64`):
  - Конструкторы: `from_nanos`, `from_micros`, `from_millis`, `from_seconds`,
    `from_minutes`, `from_hours`, `from_days`.
  - Проверяемые варианты: `checked_from_micros`, `checked_from_millis`, `checked_from_seconds`.
  - Методы: `as_nanos`, `as_micros`, `as_millis`, `as_seconds`, `as_seconds_f64`.
  - Арифметика: `checked_add`, `checked_sub`, `saturating_add`, `saturating_sub`,
    `try_add`, `try_sub`.
  - Свойства: `is_positive`, `is_negative`, `is_zero`, `abs`.
  - Реализованы трейты: `Add`, `AddAssign`, `Sub`, `SubAssign`, `Neg`, `Display`.

- **Тип `Time<S>`** — параметризованная временная метка с наносекундной точностью
  (`u64`).
  - Общие методы: `from_nanos`, `from_seconds`, `checked_from_seconds`, `as_nanos`,
    `as_seconds`, `as_seconds_f64`.
  - Арифметика с `Duration`: `checked_add`, `checked_sub_duration`, `saturating_add`,
    `saturating_sub_duration`, `try_add`, `try_sub_duration`.
  - Разность `Time - Time` возвращает `Duration`.
  - Реализованы трейты: `Add<Duration>`, `Sub<Duration>`, `AddAssign`, `SubAssign`,
    `Sub<Time>`, `PartialOrd`, `Ord`, `Debug`, `Display`.

- **Шкалы времени (`scale`)** — маркерные типы для GPS, GLONASS, Galileo, BeiDou,
  TAI, UTC.
  - Каждая шкала определяет: имя, смещение относительно TAI, календарную эпоху,
    стиль отображения.
  - `OffsetToTai::Fixed` для шкал с постоянным смещением (GPS, Galileo, BeiDou,
    TAI).
  - `OffsetToTai::Contextual` для UTC и GLONASS (требуют leap seconds).

- **Эпохи и календарная арифметика (`epoch`)**.
  - Тип `CivilDate` для пролептической григорианской даты.
  - Константы эпох: `GPS_EPOCH`, `GLONASS_EPOCH`, `GALILEO_EPOCH`, `BEIDOU_EPOCH`,
    `TAI_EPOCH`, `UNIX_EPOCH`.
  - Константы смещений между эпохами (в днях, секундах, наносекундах).
  - `const fn` для вычисления разницы между датами на этапе компиляции.

- **Специфичные конструкторы для GPS и GLONASS**.
  - `Time<Gps>::from_week_tow(week, tow_s)` и методы `week()`, `tow_seconds()`,
    `sub_second_nanos()`.
  - `Time<Glonass>::from_day_tod(day, tod_s)` и методы `day()`, `tod_seconds()`.

- **Leap seconds (`leap`)** — поддержка конверсий через таблицу високосных секунд.
  - Тип `LeapEntry` с полями `tai_nanos` и `tai_minus_utc`.
  - Тип `LeapSeconds` со статической встроенной таблицей (19 записей, от 1980 до
    2017).
  - Трейт `LeapSecondsProvider` для кастомных источников (blanket impl для `&P`).
  - Функции конверсии:
    - `gps_to_utc`, `utc_to_gps` (требуют `LeapSecondsProvider`).
    - `glonass_to_utc`, `utc_to_glonass` (константный сдвиг, без leap seconds).
    - `gps_to_glonass`, `glonass_to_gps` (через UTC).
  - Тесты для граничных переходов leap second (1998→1999, 2016→2017).

- **Тип ошибок `GnssTimeError`** с вариантами:
  - `Overflow` — арифметическое переполнение.
  - `InvalidInput` — неверный аргумент (например, TOW вне диапазона).
  - `LeapSecondsRequired` — требуется контекст leap seconds.

- **Форматирование `Display`** в зависимости от шкалы:
  - `WeekTow` (GPS, Galileo, BeiDou): `"GPS 2345:432000.000"`.
  - `DayTod` (GLONASS): `"GLO 10512:43200.000"`.
  - `Simple` (TAI, UTC): `"TAI +1000000000s 0ns"`.

- **Examples**:
  - `basic_usage.rs` — создание меток, арифметика, saturating операции.
  - `gps_week_tow.rs` — работа с GPS неделями и TOW.
  - `glonass_day_tod.rs` — работа с GLONASS днями и TOD.
  - `scale_conversion.rs` — конвертация между шкалами через TAI.
  - `display_formats.rs` — демонстрация разных форматов вывода.

- **Тесты** — покрытие всех ключевых функций, включая проверки на переполнение,
  граничные случаи leap seconds, round-trip конверсии.

### Documentation

- Added `README.md` with overview, scale table, and usage example.
- Added detailed documentation comments in modules `duration`, `epoch`, `scale`,
  `time`,
  `leap`.
- Added `#![deny(missing_docs)]` (optional, if enabled).

### Performance

- All types are 8 bytes (`Duration` — `i64`, `Time<S>` — `u64`).
- `Time<S>` and `Duration` are `repr(transparent)`.
- Conversions via TAI use integer arithmetic with no allocations.
- Leap second table lookup uses binary search over a `&'static` slice.

[Unreleased]: https://github.com/MiCkEyZzZ/gnss-time/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/MiCkEyZzZ/gnss-time/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MiCkEyZzZ/gnss-time/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MiCkEyZzZ/gnss-time/releases/tag/v0.1.0
