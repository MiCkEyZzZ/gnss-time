# Changelog

All notable changes to **gnss-time** are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **docs**
  - добавил описание `ARCHITECTURE.md`
  - добавил описание `EMBEDDED.md`
  - добавил описание `GNSS_TIME_PRIMER.md`
  - добавил описание `INVARIANTS.md`
  - добавил описание `LEAP_SECONDS.md`

- **README**
  - улучшил описание проекта, добавил дополнительные примеры
    показывающие принцип работы библиотеки

- **convert**
  - улучшил документацию по коду

- **Property-based тесты (ручная реализация)**: добавлены `tests/prop_tests.rs`
  с 9 property-тестами:
  - Roundtrip GPS→UTC→GPS для всех выборок (границы, leap seconds, равномерные точки,
    реальные эпохи)
  - Roundtrip GPS→GAL→GPS, GPS→BDT→GPS, GPS→TAI→GPS
  - Сортировка `Vec<Time<Gps>>` совпадает с сортировкой по внутреннему `u64`
  - Монотонность GPS→UTC в интервалах между leap seconds
  - Проверка GPS−UTC смещений на известных эпохах
  - Все 18 исторических leap second transitions (1981–2017)
  - Строгое возрастание GPS−UTC разности на каждом переходе

- **time.rs**:
  - добавлены дополнительные тесты для проверки на:
    - `Time<Gps>::MAX` must equal `u64::MAX` nanoseconds
    - `NANOS_PER_YEAR` sanity: 365 days worth of nanoseconds
    - `MAX` covers at least 500 years from the GPS epoch (1980-01-06)
    - checked_add near `u64::MAX`
    - checked_sub near `EPOCH`
    - saturating_add
    - saturating_sub_duration
    - try_add / try_sub_duration
    - Panicking operators panic on overflow
    - checked_elapsed near i64 boundary

### Changed

- `benches/arithmetic_bench.rs`: added `checked_add`, `checked_sub_duration`,
  `saturating_add`, `Duration` benchmarks; updated target figures
- `benches/convert_bench.rs`: added `leap_second_lookup` microbenchmark

### Fixed

- **time.rs**: исправлена опечатка "oberflow" → "overflow" в документации
- **time.rs**: добавлены новые тесты граничных случаев (`test_checked_add_one_ns_before_max_succeeds`,
  `test_saturating_add_negative_clamps_at_epoch`, и др.)

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

- **Cargo.toml**
  - обвновлена конфигурацию добавлена поддержка:
    - `default = []` - по умолчанию, нет лишних зависимостей
    - `std = []` - добавляет `std::error::Error` для `GnssTimeError`
    - `defmt = ["dep:defmt"]` - активирует `defmt::Format` для всех типов через
      `dep:` синтаксис (Cargo 1.60+), подтягивает `defmt = { version = "0.3", optional = true }`
    - `[package.metadata.docs.rs]` - docs.rs собирает с `["std","defmt"]` и
      показывает embedded targets

- **justfile**
  - улучшил конфигурацию команд добавлены новые команды:
    - `help`
    - `setup-embedded`
    - `check`
    - `check-std`
    - `check-no-std`
    - `check-no-std-defmt`
    - `lint-no-std`
    - `msrv`
    - `hack`
    - `test-host`
    - `test-no-std`
    - `ci`

### Fixed

- **time.rs**: исправлена опечатка в документации "Дипазон значений" → "Диапазон
  значений".

- **time.rs**: убран `const` у метода `as_seconds_f64` (совместимость с `no_std`).

- **leap.rs**: исправлена функция `LeapSeconds::builtin()` для совместимости с `no_std`.
  - Раньше использовалась `const fn`, которая не может обращаться к статическим
    данным.
  - Теперь возвращает ссылку на статический экземпляр `BUILTIN_LEAP_SECONDS`, что
    корректно работает в `no_std`-среде.

- **time.rs**
  - исправлена ф-я `as_seconds_f64` убрано ключевое слово `const`

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

- **Исправлен алгоритм `utc_to_gps`** — заменён однопроходный приближённый lookup
  на двухпроходный.
  Раньше на границе leap second обратная конверсия `GPS → UTC → GPS` давала ошибку
  в 1 секунду.
  Теперь roundtrip точен с погрешностью менее 1 наносекунды (фундаментальная
  неоднозначность остаётся только в самой секунде вставки).

### Documentation

- Добавлена документация к `convert` модулю с таблицей поддерживаемых конверсий
  и примерами.
- Добавлен `prelude` для удобного импорта.

## [0.1.0] — 2026-04-21

- **Тип `Duration`** — знаковый интервал времени в наносекундах (`i64`).
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

- **Примеры (`examples/`)**:
  - `basic_usage.rs` — создание меток, арифметика, saturating операции.
  - `gps_week_tow.rs` — работа с GPS неделями и TOW.
  - `glonass_day_tod.rs` — работа с GLONASS днями и TOD.
  - `scale_conversion.rs` — конвертация между шкалами через TAI.
  - `display_formats.rs` — демонстрация разных форматов вывода.

- **Тесты** — покрытие всех ключевых функций, включая проверки на переполнение,
  граничные случаи leap seconds, round-trip конверсии.

### Changed

- Нет (первый выпуск).

### Fixed

- Нет (первый выпуск).

### Removed

- Нет.

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
