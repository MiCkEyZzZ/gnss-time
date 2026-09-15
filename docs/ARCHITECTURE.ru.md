# Архитектура

Внутренняя структура `gnss-time`.

## Модульная структура

```text
src/
├── tables/
│   ├── leap_seconds.rs  — BUILTIN_TABLE (19 записей GPS-эры)
│   └── mod.rs
├── convert.rs      — трейты IntoScale / IntoScaleWith + все реализации
├── civil.rs        — CivilDateTime (ISO 8601 / RFC 3339, из Time<Utc>)
├── duration.rs     — Duration (интервал со знаком в наносекундах)
├── epoch.rs        — CivilDate, константные эпохи, Unix-офсеты
├── error.rs        — GnssTimeError
├── leap.rs         — LeapSecondsProvider, LeapSeconds, все функции конвертации
├── lib.rs          — корень крейта, #![no_std], pub use-реэкспорты
├── matrix.rs       — ConversionMatrix, ScaleId, ConversionKind
├── prelude.rs      — удобные реэкспорты
├── scale.rs        — sealed-трейт TimeScale + 6 маркерных типов
├── serde_impls.rs  — Serialize/Deserialize для Time<S>, Duration, DurationParts
│                     (только когда feature = "serde")
└── time.rs         — структура Time<S>, конструкторы, арифметика, Unix-методы
```

## Ключевая инварианта: TAI как центральная точка (pivot) для конвертаций с фиксированным смещением

Любая конвертация с фиксированным смещением относительно TAI проходит через
TAI:

```text
T_tai = T_self + S::OFFSET_TO_TAI
T_target = T_tai - Target::OFFSET_TO_TAI
```

Это означает, что все попарные конвертации между шкалами с фиксированным
TAI-смещением выводятся из единого согласованного набора смещений
относительно TAI. Ошибки на единицу (off-by-one) между отдельными парами
шкал невозможны.

Две контекстные шкалы — UTC и GLONASS — используют `OffsetToTai::Contextual`,
поэтому у них нет постоянной TAI-связи. Тем не менее, сама конвертация
GLONASS ↔ UTC является фиксированной (`IntoScale`): GLONASS определён через
UTC(SU) = UTC + 3 ч и поэтому пересчитывается через постоянный сдвиг эпохи.

Смещения (в наносекундах) — константы времени компиляции, встроены в enum
`OffsetToTai`:

| Шкала   | OFFSET_TO_TAI      |
| ------- | ------------------ |
| GPS     | +19_000_000_000 ns |
| Galileo | +19_000_000_000 ns |
| BeiDou  | +33_000_000_000 ns |
| TAI     | 0                  |
| UTC     | Контекстный        |
| GLONASS | Контекстный        |

## Паттерн sealed-трейта

`TimeScale` — **sealed-трейт** — его нельзя реализовать вне этого крейта:

```rust
mod private { pub trait Sealed {} }

pub trait TimeScale: private::Sealed + ... { ... }
```

Это не даёт пользователю создать новую «псевдо-шкалу времени», которая
незаметно сломает все конвертации. Набор поддерживаемых шкал фиксирован.

## Представление в памяти

`Time<S>` занимает ровно 8 байт (совпадает с `u64`):

```rust
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,  // ZST, память не занимает
}
```

- Маркерные типы `S` (`Gps`, `Glonass`, …) — тоже Zero-Sized Types
- Никаких heap-аллокаций
- Вся типизация существует исключительно на этапе компиляции

## Архитектура високосных секунд (leap seconds)

### Почему явный контекст?

```rust
// ❌ Скрытое состояние — откуда берутся високосные секунды?
let utc = gps.to_utc();

// ✅ Явный контекст — тестируемо, no_std-совместимо, детерминированно
let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
```

### Двухпроходный алгоритм UTC → GPS

Наивная конвертация UTC → GPS даёт ошибку ±1 секунда вблизи момента вставки
високосной секунды. Библиотека использует двухпроходный алгоритм:

**Проход 1:** TAI вычисляется приближённо, в предположении GPS − UTC = 0

**Проход 2:** уточнение по числу високосных секунд из первого прохода

Это устраняет ошибку на границах всех исторических вставок високосных
секунд. Встроенная таблица високосных секунд содержит 19 записей: исходное
состояние GPS-эры (TAI − UTC = 19 с) плюс 18 последующих переходов високосных
секунд, вплоть до TAI − UTC = 37 с (2017-01-01). Таблица и её тесты покрывают
все 18 переходов GPS-эры.

## Интероперабельность с временем Unix

`Time<Utc>` считает наносекунды от **1972-01-01** (эпоха UTC), тогда как
время Unix отсчитывается от **1970-01-01**. Разница составляет
`UTC_EPOCH_UNIX_OFFSET_S = 63_072_000 с` (730 дней):

```text
unix_seconds    = utc_seconds_from_1972 + UTC_EPOCH_UNIX_OFFSET_S
utc_from_1972   = unix_seconds          - UTC_EPOCH_UNIX_OFFSET_S
```

Это чистое отображение «счётчик ↔ счётчик»: `Time<Utc>` хранит линейный счёт
наносекунд без разрывов из-за високосных секунд. Високосные секунды
применяются только при конвертации между шкалами времени (см. выше) — не в
этом Unix-отображении, которое работает с внутренним представлением, а не с
календарём, учитывающим високосные секунды.

Предоставленные методы:

| Тип          | Метод                                       |
| ------------ | -------------------------------------------- |
| `Time<Utc>`  | `from_unix_seconds(i64) -> Result<Self>`     |
| `Time<Utc>`  | `from_unix_nanos(i64)   -> Result<Self>`     |
| `Time<Utc>`  | `as_unix_seconds() -> i64`                   |
| `Time<Utc>`  | `as_unix_nanos()   -> i64`                   |
| `Time<Gps>`  | `from_unix_seconds(i64, P) -> Result<Self>`  |
| `Time<Gps>`  | `as_unix_seconds(P) -> Result<i64>`          |

## Поддержка Serde (feature = "serde")

Включение:

```toml
gnss-time = { version = "0.8", features = ["serde"] }
```

### Форматы

#### `Time<S>`

**Human-readable** (JSON, TOML, YAML):

```json
{ "scale": "GPS", "nanos": 1356566418000000000 }
```

Поле `scale` проверяется при десериализации — попытка десериализовать
`{ "scale": "UTC", ... }` в `Time<Gps>` возвращает ошибку.

**Compact** (postcard, bincode, MessagePack): сырое `u64` наносекунд без тега
шкалы. Шкала переносится системой типов.

#### `Duration`

| Формат         | Форма                          |
| -------------- | ------------------------------ |
| Human-readable | `{ "nanos": -7000000000 }`     |
| Compact        | сырой `i64`                    |

#### `DurationParts`

| Формат         | Форма                                 |
| -------------- | -------------------------------------- |
| Human-readable | `{ "seconds": 5, "nanos": 500000000 }` |
| Compact        | кортеж из 2 элементов `[u64, u32]`     |

`DurationParts` — отдельный беззнаковый составной тип
(`seconds: u64`, `nanos: u32`), используемый конструкторами
недель/дней GNSS. Он никогда не кодирует знак — отрицательные интервалы
существуют только в самом `Duration` (compact `i64`).

### Принципы реализации

- **Без proc-macro** — реализации написаны вручную через visitor-API serde
- **Совместимо с no_std** — `serde` подключается с `default-features = false`
- `is_human_readable()` определяет формат во время выполнения — одна
  реализация работает и с JSON, и с postcard
- Ошибки проверки шкал не требуют `alloc` — используется `fmt::Display`

```rust
// Пример — JSON-раундтрип
let gps = Time::<Gps>::from_seconds(1_356_566_418);
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

let back: Time<Gps> = serde_json::from_str(&json).unwrap();
assert_eq!(gps, back);

// Пример — postcard-раундтрип
let bytes = postcard::to_allocvec(&gps).unwrap();
let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();
assert_eq!(gps, back);
```

Примечание: сам serde-код `gnss-time` остаётся no_std-совместимым; вызов
`postcard::to_allocvec()` в примере дополнительно требует `alloc` (feature
`alloc` крейта `postcard`).

## Feature-флаги

| Feature | Эффект                                                     |
| ------- | ----------------------------------------------------------- |
| (нет)   | Чистый `no_std`, без внешних зависимостей                   |
| `std`   | `impl std::error::Error for GnssTimeError`                  |
| `serde` | `Serialize`/`Deserialize` для всех публичных типов          |
| `alloc` | Зарезервирован (no-op) — heap-сообщения об ошибках в serde планируются |
| `defmt` | `impl defmt::Format` для всех публичных типов               |

## Дизайн трейтов конвертации

```rust
// Фиксированное смещение — GPS ↔ TAI, GPS ↔ Galileo, GLONASS ↔ UTC
pub trait IntoScale<Target: TimeScale>: Sized {
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}

// Контекстные конвертации — GPS ↔ UTC, GPS ↔ GLONASS и т.д.
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    fn into_scale_with<P: LeapSecondsProvider>(self, ls: P)
        -> Result<Time<Target>, GnssTimeError>;
    fn into_scale_with_checked<P: LeapSecondsProvider>(self, ls: P)
        -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}
```

`ConvertResult<T>` добавляет сигнал о попадании в окно неоднозначности
високосной секунды.

## Гарантии CI

| Проверка                          | Инструмент                                                       |
| ---------------------------------- | ---------------------------------------------------------------- |
| Нет unsafe-кода                    | `#![forbid(unsafe_code)]`                                        |
| Нет недокументированного API       | `#![deny(missing_docs)]`                                         |
| Сборка для embedded-целей          | `cargo check --target thumbv7em-none-eabihf`                     |
| Размер типа = 8 байт               | юнит-тест `test_size_equals_u64`                                 |
| Безопасная арифметика              | `-D warnings` + отсутствие `#[allow(arithmetic_overflow)]`        |
| Serde-раундтрип (JSON)             | тесты в `src/serde_impls.rs`                                     |
| Serde-раундтрип (postcard)         | тесты в `src/serde_impls.rs`                                     |