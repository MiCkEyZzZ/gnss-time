# Changelog

All notable changes to **gnss-time** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

- **API breaking change**: replaced `f64` fractional seconds with `DurationParts`
  for all constructors:
  - `Time<Gps>::from_week_tow(week, tow)` теперь принимает `DurationParts`
  - `Time<Glonass>::from_day_tod(day, tod)` теперь принимает `DurationParts`
  - Добавлена валидация на этапе конструирования: `seconds` и `nanos` проверяются
    на диапазон
  - Устранена недетерминированность, связанная с `f64`
- **Новый тип `DurationParts`**:
  - Поля `seconds: u64` и `nanos: u32`
  - Конструктор `new()` с валидацией `nanos < 1_000_000_000`
  - Метод `as_nanos() -> u128` для преобразования в наносекунды
- **Обновлены все примеры** (`examples/`):
  - `basic_usage.rs`, `gps_week_tow.rs`, `glonass_day_tod.rs`
  - `convert_basic.rs`, `convert_contextual.rs`, `chain_conversion.rs`
  - `display_formats.rs`, `dynamic_conversion.rs`, `embedded_safe_arithmetic.rs`
  - `glonass_receiver.rs`, `gps_time_operations.rs`, `log_stream.rs`
  - `matrix_inspection.rs`, `multi_constellation.rs`, `no_domain_mixing.rs`
  - `no_std_example.rs`, `receiver_timestamp.rs`, `scale_conversion.rs`
  - `sync_alignment.rs`
- **Обновлены интеграционные тесты** (`tests/`):
  - `glonass_test.rs` — все конструкторы переписаны на `DurationParts`
  - `roundtrip_test.rs` — все тесты roundtrip обновлены
  - `time_integration_test.rs` — адаптирован под новый API
- **Обновлены бенчмарки** (`benches/`):
  - `arithmetic_bench.rs` — без изменений (не использует конструкторы)
  - `convert_bench.rs` — обновлены вызовы `from_week_tow` с `DurationParts`
  - `time_bench.rs` — обновлены конструкторы
- **Документация**:
  - Добавлена полная документация для `DurationParts`
  - Обновлены примеры в doc-комментариях всех модулей

- CI architecture:
  - embedded checks extracted into reusable workflow (`embedded.yml`)
  - improved caching strategy (feature-aware cache keys)
  - stricter validation (`-D warnings`, clippy on all feature combinations)
- `benches/arithmetic_bench.rs`: added `checked_add`, `checked_sub_duration`,
  `saturating_add`, `Duration` benchmarks; updated target figures
- `benches/convert_bench.rs`: added `leap_second_lookup` microbenchmark

## [0.3.0] — 2026-04-27

### Added

- **Бенчмарки (#TIME-12)**: добавлены `benches/arithmetic_bench.rs` и `benches/convert_bench.rs`.
  - Доказывают zero-cost абстракции: `Time<Gps> + Duration` (512 ps) наравне с
    `u64 + u64` (517 ps).
  - Конверсии без leap seconds: ~0.8–0.9 нс.
  - Конверсия `GPS → UTC` с leap seconds: ~9.5 нс (менее 10 нс).
  - Используется `criterion` с HTML-отчётами.

- **time.rs**: добавлена константа `Time::MIN` (синоним `EPOCH`) для симметрии
  с `MAX`.

- **time.rs**: добавлена документация о диапазоне значений `Time<S>` (~584 года
  от эпохи, для GPS до 2554 года).

- **time.rs**: добавлен тест `test_time_max_behavior` для проверки поведения
  вблизи `u64::MAX`.

- **.github/workflows/embedded.yml**: добавлена проверка `clippy::arithmetic_overflow`
  в lint job.

- **Добавлен шаблон Issue `enhancement.yml`** для предложений по улучшению
  существующей функциональности.
  - Категории: производительность, API, конверсии шкал времени, leap seconds,
    embedded/no_std, форматирование, рефакторинг, тестирование, документация.

- **Добавлен файл `CODEOWNERS`** для автоматического назначения владельцев на разные
  части репозитория.
  - Определяет ответственность за код (`/src/`), тесты (`/tests/`), бенчмарки
    (`/benches/`), примеры (`/examples/`), CI/CD (`/.github/workflows/`), документацию
    (`/docs/`) и корневые файлы.
  - Используется GitHub для автоматического ревью и назначения проверяющих на
    Pull Request.

- **Добавлен шаблон Pull Request** (`.github/pull_request_template.md`).
  - Содержит структурированный чеклист для проверки изменений: указание scope,
    описание изменений, способов тестирования.
  - Включает обязательные проверки: `cargo fmt`, `taplo format`, `cargo clippy`,
    `cargo test`, документацию и обновление CHANGELOG.

- **CI: добавлен GitHub Actions workflow для проверки семантического формата
  заголовков Pull Request** (`.github/workflows/semantic-pull-request.yml`).
  - Автоматически проверяет заголовки PR на соответствие формату `type(scope?):
описание`.
  - Поддерживаемые типы: `feat`, `fix`, `docs`, `chore`, `perf`, `refactor`,
    `test`, `ci`, `build`, `style`.
  - Работает только для нечерновиков PR (draft-игнорируются).
  - При некорректном заголовке автоматически оставляет комментарий с пояснением.

- **.github/workflows**
  - добавлен файл `embedded.yml` для проверки рамеров типов, сборка под
    `thumbv7em-none-eabihf`, `thumbv7em-none-eabi`, `riscv32imac-unknown-none-elf`,
    хостовые тесты, clippy.

- **.cargo**
  - добавлен файл с конфигурации `config.toml` для кросс-компиляции:
    - `thumbv7em-none-eabihf`,
    - `thumbv7em-none-eabi`,
    - `thumbv6m-none-eabi`,
    - `riscv32imac-unknown-none-elf`,
    - `riscv32i-unknown-none-elf`,
    - `opt-level=s` - минимальный размер для flash-ограниченных устройств
    - `codegen-units=1` - лучшая оптимизация
    - `-C link-arg=-Tlink.x` для Cortex-M (нужен linker script из `cortex-m-rt`)
    - `-D warnings`- предупреждения как ошибки в embedded CI

- **tests**
  - добавлены тесты `no_std_compact.rs` проверяющие на отсутствие `Drop`, `Copy-семантика`,
    `const fn` в static-контексте, 8-битовое выравнивание для DMA, без аллокаций
    в conversion paths, `core::fmt` без std, проверка `#![forbid(unsafe_code)]`

- **time.rs**
  - добавлена имлементация `impl<S: TimeScale> defmt::Format for Time<S>` для
    `#[cfg(feature = "defmt")]`

- **error.rs**
  - добавлена имплементация `impl defmt::Format for GnssTimeError` для
    `#[cfg(feature = "defmt")]`

### Changed

- `Cargo.toml`: bumped to `0.3.0`; added `defmt = ["dep:defmt"]` with
  `dep:` syntax (Cargo 1.60+); added `[package.metadata.docs.rs]` for
  docs.rs targets and features.
- `justfile`: added `setup-embedded`, `check-std`, `check-no-std`,
  `check-no-std-defmt`, `lint-no-std`, `msrv`, `hack`, `test-host`,
  `test-no-std`, `ci` commands.

### Fixed

- `leap.rs`: `LeapSeconds::builtin()` now returns `&'static LeapSeconds`
  (was `const fn` — incompatible with `no_std` static data access).
- `time.rs`: removed `const` from `as_seconds_f64` (floating-point ops
  are not `const` in stable Rust 1.75).

## [0.2.0] — 2026-04-26

### Added

- **Полная матрица конверсий (`matrix`)**:
  - Тип `ScaleId` для идентификации шкал времени в рантайме (GPS, GLONASS, Galileo,
    BeiDou, TAI, UTC).
  - Тип `ConversionKind` – классификация преобразований (Fixed, Identity, EpochShift,
    Contextual, SameScale).
  - Структура `ConversionMatrix` – проверка совместимости и статистика по графу
    конверсий.
  - Константы смещений относительно TAI: `TAI_OFFSET_GPS_NS`, `TAI_OFFSET_GALILEO_NS`,
    `TAI_OFFSET_BEIDOU_NS`, `TAI_OFFSET_TAI_NS`, `GLONASS_UTC_EPOCH_SHIFT_NS`.
  - Функция `beidou_via_gps_to_glonass_via_utc` – пример последовательного преобразования
    через все шкалы.
  - Тесты для проверки симметричности и классификации всех 30 внедиагональных путей.

- **Расширенные возможности конверсий в `leap` и `convert`**:
  - Функции `galileo_to_utc`, `galileo_to_glonass`, `beidou_to_utc`, `beidou_to_glonass`,
    а также соответствующие обратные преобразования `utc_to_galileo`, `utc_to_beidou`.
  - Реализации трейтов `IntoScale` и `IntoScaleWith` для всех пар шкал, включая
    Galileo ↔ GLONASS, BeiDou ↔ GLONASS, Galileo ↔ UTC, BeiDou ↔ UTC.
  - Полная поддержка 6×6 матрицы конверсий (всего 30 направлений).

- **Исправлена опечатка** в doctest `matrix.rs` (метод `needs_leap_seconds` и число
  контекстных путей 16 вместо 22).

- **Новые примеры**:
  - `matrix_inspection.rs` – вывод матрицы конверсий.
  - `dynamic_conversion.rs` – динамическая конверсия (рантайм).
  - `chain_conversion.rs` – сквозная цепочка BeiDou → TAI.

- **GLONASS‑специфичные методы** (`Time<Glonass>`):
  - `sub_second_nanos()` – наносекундная доля текущей секунды.
  - `day_of_week()` – день недели по ISO (1 = Monday … 7 = Sunday), основан на
    эпохе 1996-01-01 (понедельник).
  - `is_weekend()` – возвращает `true` для субботы или воскресенья.

- **Интеграционные тесты GLONASS** (`tests/glonass_test.rs`):
  - Проверка постоянного сдвига GLO ↔ UTC (без leap seconds).
  - Roundtrip GLO → UTC → GLO и GLO → GPS → GLO.
  - Проверка корректности `day_of_week()` на известных датах.
  - Поведение на границе leap second (2017-01-01).

- **Единый конверсионный API (`convert`)**.
  - Трейт `IntoScale<Target>` для конверсий с фиксированным смещением (GPS↔TAI,
    GPS↔Galileo, GPS↔BeiDou, GLO↔UTC).
  - Трейт `IntoScaleWith<Target>` для контекстных конверсий (GPS↔UTC, GPS↔GLO) с
    явной передачей `LeapSecondsProvider`.
  - Тип `ConvertResult<T>` для обработки неоднозначного 1-секундного окна при
    вставке leap second.
  - Метод `into_scale_with_checked` для детектирования момента внутри leap second.

- **Модуль `prelude`** — удобный импорт самых часто используемых типов:

  ```rust
  use gnss_time::prelude::*;
  ```

- **Новые примеры (`examples/`)**:
  - `convert_basic.rs` — демонстрация конверсий с фиксированным смещением (без
    leap seconds).
  - `convert_contextual.rs` — демонстрация GPS↔UTC конверсий с leap seconds и
    детекцией неоднозначности.

- **Интеграционные тесты (`tests/`)**:
  - `roundtrip_test.rs` — проверка roundtrip точности для всех шкал, 18 переходов
    leap seconds, известные RINEX эпохи.
  - `time_integration_test.rs` — комплексные сценарии использования.

### Fixed

- `utc_to_gps`: replaced single-pass approximation with a two-pass algorithm.
  Roundtrip `GPS → UTC → GPS` is now exact (< 1 ns) at all 18 GPS-era
  leap second boundaries.

### Documentation

- Добавлена документация к `convert` модулю с таблицей поддерживаемых конверсий
  и примерами.
- Добавлен `prelude` для удобного импорта.

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

- Добавлен `README.md` с описанием, таблицей шкал и примером.
- Добавлены подробные комментарии в модулях `duration`, `epoch`, `scale`, `time`,
  `leap`.
- Добавлены `#![deny(missing_docs)]` (опционально, если включите).

### Performance

- Все типы занимают 8 байт (`Duration` — `i64`, `Time<S>` — `u64`).
- `Time<S>` и `Duration` — `repr(transparent)`.
- Конверсии через TAI используют целочисленную арифметику без аллокаций.
- Поиск в таблице leap seconds — бинарный поиск по `&'static` слайсу.

[Unreleased]: https://github.com/MiCkEyZzZ/gnss-time/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/MiCkEyZzZ/gnss-time/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MiCkEyZzZ/gnss-time/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MiCkEyZzZ/gnss-time/releases/tag/v0.1.0
