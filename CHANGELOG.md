# Changelog

All notable changes to **gnss-time** are documented in this file.

## [Unreleased] — 0000-00-00

### Added

- **Тип `Duration`** — знаковый интервал времени в наносекундах (`i64`).
    - Конструкторы: `from_nanos`, `from_micros`, `from_millis`, `from_seconds`,
      `from_minutes`, `from_hours`, `from_days`.
    - Проверяемые варианты: `checked_from_micros`, `checked_from_millis`, `checked_from_seconds`.
    - Методы: `as_nanos`, `as_micros`, `as_millis`, `as_seconds`, `as_seconds_f64`.
    - Арифметика: `checked_add`, `checked_sub`, `saturating_add`, `saturating_sub`,
      `try_add`, `try_sub`.
    - Свойства: `is_positive`, `is_negative`, `is_zero`, `abs`.
    - Реализованы трейты: `Add`, `AddAssign`, `Sub`, `SubAssign`, `Neg`, `Display`.

- **Тип `Time<S>`** — параметризованная временная метка с наносекундной точностью (`u64`).
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
    - Тип `LeapSeconds` со статической встроенной таблицей (19 записей, от 1980 до 2017).
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
