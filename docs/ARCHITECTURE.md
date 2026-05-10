# Architecture

Internal design of `gnss-time`.

## Module layout

```text
src/
├── tables/
│   ├── leap_seconds.rs  — BUILTIN_TABLE (19 GPS-era entries)
│   └── mod.rs
├── convert.rs      — трейты IntoScale / IntoScaleWith + все реализации
├── duration.rs     — Duration (знаковый интервал в наносекундах)
├── epoch.rs        — CivilDate, константные смещения эпох, Unix offsets
├── error.rs        — GnssTimeError
├── leap.rs         — LeapSecondsProvider, LeapSeconds, все функции преобразований
├── lib.rs          — корень крейта, #![no_std], pub use реэкспорты
├── matrix.rs       — ConversionMatrix, ScaleId, ConversionKind
├── prelude.rs      — удобные re-export'ы
├── scale.rs        — sealed-трейт TimeScale + 6 маркерных типов
├── serde_impls.rs  — Serialize/Deserialize для Time<S>, Duration, DurationParts
│                     (только при feature = "serde")
└── time.rs         — структура Time<S>, конструкторы, арифметика, Unix-методы
```

## Core invariant: TAI as the universal pivot

Любое преобразование с фиксированным смещением проходит через TAI:

```text
T_tai = T_self + S::OFFSET_TO_TAI
T_target = T_tai - Target::OFFSET_TO_TAI
```

Это означает, что все попарные преобразования выводятся из единого согласованного
набора смещений относительно TAI. Нет возможности получить ошибки вида off-by-one
между отдельными парами шкал.

Смещения (в наносекундах) — это константы времени компиляции, встроенные в enum
`OffsetToTai`:

| Scale   | OFFSET_TO_TAI      |
| ------- | ------------------ |
| GPS     | +19_000_000_000 ns |
| Galileo | +19_000_000_000 ns |
| BeiDou  | +33_000_000_000 ns |
| TAI     | 0                  |
| UTC     | Contextual         |
| GLONASS | Contextual         |

## Паттерн sealed trait

`TimeScale` — закрытый (sealed) трейт, его нельзя реализовать вне этого крейта:

```rust
mod private { pub trait Sealed {} }

pub trait TimeScale: private::Sealed + ... { ... }
```

Это предотвращает ситуацию, когда пользователь создаёт новую «псевдошкалу времени»,
которая незаметно ломает все преобразования. Множество поддерживаемых шкал фиксировано.

## Представление в памяти

`Time<S>` — ровно 8 байт (идентично `u64`):

```rust
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,  // нулевого размера
}
```

- Маркерные типы `S` (`Gps`, `Glonass`, …) тоже нулевого размера
- Нет выделений памяти в куче
- Вся типизация существует только на этапе компиляции

## Архитектура високосных секунд

### Почему явный контекст?

```rust
// ❌ Скрытое состояние — откуда берутся leap seconds?
let utc = gps.to_utc();

// ✅ Явный контекст — тестируемо, совместимо с no_std, детерминированно
let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
```

### Двухпроходный алгоритм UTC → GPS

Наивное преобразование UTC → GPS даёт ошибку ±1 секунда рядом с моментом вставки
високосной секунды. В библиотеке используется двухпроходный алгоритм:

**Проход 1:** приближённо вычисляется TAI, предполагая GPS − UTC = 0

**Проход 2:** уточнение с использованием количества leap seconds из первого прохода

Это устраняет ошибку на границах всех исторических вставок високосных секунд.
Тесты `utc_to_gps` покрывают все 18 переходов эпохи GPS.

## Unix time interoperability

`Time<Utc>` считает наносекунды от **1972-01-01** (UTC epoch), тогда как Unix time
считает от **1970-01-01**. Разница — `UTC_EPOCH_UNIX_OFFSET_S = 63_072_000 с`
(730 дней).

```text
unix_seconds    = utc_seconds_from_1972 + UTC_EPOCH_UNIX_OFFSET_S
utc_from_1972   = unix_seconds          - UTC_EPOCH_UNIX_OFFSET_S
```

Предоставляемые методы:

| Тип         | Метод                                       |
| ----------- | ------------------------------------------- |
| `Time<Utc>` | `from_unix_seconds(i64) -> Result<Self>`    |
| `Time<Utc>` | `from_unix_nanos(i64)   -> Result<Self>`    |
| `Time<Utc>` | `as_unix_seconds() -> i64`                  |
| `Time<Utc>` | `as_unix_nanos()   -> i64`                  |
| `Time<Gps>` | `from_unix_seconds(i64, P) -> Result<Self>` |
| `Time<Gps>` | `as_unix_seconds(P) -> Result<i64>`         |

## Serde поддержка (feature = "serde")

Подключение:

```toml
gnss-time = { version = "0.5", features = ["serde"] }
```

### Форматы

#### `Time<S>`

**Human-readable** (JSON, TOML, YAML):

```json
{ "scale": "GPS", "nanos": 1356566418000000000 }
```

Поле `scale` валидируется при десериализации — попытка десериализовать
`{ "scale": "UTC", ... }` в `Time<Gps>` вернёт ошибку.

**Compact** (postcard, bincode, MessagePack): сырой `u64` наносекунд без тега шкалы.
Шкала несёт система типов.

#### `Duration`

| Формат         | Вид                        |
| -------------- | -------------------------- |
| Human-readable | `{ "nanos": -7000000000 }` |
| Compact        | raw `i64`                  |

#### `DurationParts`

| Формат         | Вид                                    |
| -------------- | -------------------------------------- |
| Human-readable | `{ "seconds": 5, "nanos": 500000000 }` |
| Compact        | 2-element tuple `[u64, u32]`           |

### Принципы реализации

- **Нет proc-macro** — реализации написаны вручную через `serde` visitor API
- **no_std совместимо** — `serde` подключается с `default-features = false`
- `is_human_readable()` определяет формат во время выполнения — одна реализация
  работает как с JSON, так и с postcard
- Ошибки масштабирования не требуют `alloc` — используется `fmt::Display`

```rust
// Пример — JSON round-trip
let gps = Time::<Gps>::from_seconds(1_356_566_418);
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

let back: Time<Gps> = serde_json::from_str(&json).unwrap();
assert_eq!(gps, back);

// Пример — postcard round-trip
let bytes = postcard::to_allocvec(&gps).unwrap();
let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();
assert_eq!(gps, back);
```

## Feature-флаги

| Feature | Effect                                             |
| ------- | -------------------------------------------------- |
| (none)  | Чистый `no_std`, без внешних зависимостей          |
| `std`   | `impl std::error::Error for GnssTimeError`         |
| `serde` | `Serialize`/`Deserialize` для всех публичных типов |
| `alloc` | Heap-строки в serde error messages                 |
| `defmt` | `impl defmt::Format` для всех публичных типов      |

## Дизайн трейтов преобразования

```rust
// Фиксированное смещение — GPS ↔ TAI, GPS ↔ Galileo, GLONASS ↔ UTC
pub trait IntoScale<Target: TimeScale>: Sized {
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}

// Контекстные преобразования — GPS ↔ UTC, GPS ↔ GLONASS и т.д.
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    fn into_scale_with<P: LeapSecondsProvider>(self, ls: P)
        -> Result<Time<Target>, GnssTimeError>;
    fn into_scale_with_checked<P: LeapSecondsProvider>(self, ls: P)
        -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}
```

`ConvertResult<T>` добавляет сигнал о попадании в окно неоднозначности
високосной секунды.

## CI guarantees

| Check                        | Tool                                                       |
| ---------------------------- | ---------------------------------------------------------- |
| Нет небезопасного кода       | `#![forbid(unsafe_code)]`                                  |
| Нет недокументированного API | `#![deny(missing_docs)]`                                   |
| Сборка под embedded-цели     | `cargo check --target thumbv7em-none-eabihf`               |
| Размер типов = 8 байт        | unit-тест `test_size_equals_u64`                           |
| Безопасная арифметика        | `-D warnings` + отсутствие `#[allow(arithmetic_overflow)]` |
| Serde roundtrip (JSON)       | тесты в `src/serde_impls.rs`                               |
| Serde roundtrip (postcard)   | тесты в `src/serde_impls.rs`                               |
