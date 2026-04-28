# Архитектура

Внутренний дизайн `gnss-time`.

## Структура модулей

```text
src/
├── tables/
│   └── leap_seconds.rs  — BUILTIN_TABLE (19 записей эпохи GPS)
├── convert.rs      — трейты IntoScale / IntoScaleWith + все реализации
├── duration.rs     — Duration (знаковый интервал в наносекундах)
├── epoch.rs        — CivilDate, константные смещения эпох
├── error.rs        — GnssTimeError
├── leap.rs         — LeapSecondsProvider, LeapSeconds, все функции преобразований
├── lib.rs          — корень крейта, #![no_std], pub use реэкспорты
├── matrix.rs       — ConversionMatrix, ScaleId, ConversionKind
├── prelude.rs      — удобные re-export'ы
├── scale.rs        — sealed-трейт TimeScale + 6 маркерных типов
└── time.rs         — структура Time<S>, конструкторы, арифметика
```

## Основной инвариант: TAI как универсальная опорная точка

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

`Time<S>` определяется так:

```rust
#[repr(transparent)]  // неявно, благодаря одному полю
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,
}
```

- `PhantomData<S>` не занимает памяти. `Time<S>` — ровно 8 байт
- Маркерные типы `S` (`Gps`, `Glonass`, ...) также нулевого размера
- Нет выделений памяти в куче. Вся типизация существует только на этапе компиляции

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

**Проход 1:** приближённо вычисляется TAI, предполагая GPS - UTC = 0

**Проход 2:** уточнение с использованием количества leap seconds из первого прохода

Это устраняет ошибку на границах всех исторических вставок високосных секунд.
Тесты `utc_to_gps` покрывают все 18 переходов эпохи GPS.

## Feature-флаги

| Feature | Эффект                                        |
| ------- | --------------------------------------------- |
| (none)  | Чистый `no_std`, без внешних зависимостей     |
| `std`   | `impl std::error::Error for GnssTimeError`    |
| `defmt` | `impl defmt::Format` для всех публичных типов |

Сама библиотека никогда не подключает `std`. Тесты используют `extern crate std`
через `rust#[cfg(any(feature = "std", test))]`.

## Дизайн трейтов преобразования

Два трейта — два сценария использования:

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

`ConvertResult<T>` добавляет сигнал о попадании в окно неоднозначности високосной
секунды, не усложняя обычный путь выполнения.

## Гарантии CI

| Проверка                     | Инструмент                                                 |
| ---------------------------- | ---------------------------------------------------------- |
| Нет небезопасного кода       | `#![forbid(unsafe_code)]`                                  |
| Нет недокументированного API | `#![deny(missing_docs)]`                                   |
| Сборка под embedded-цели     | `cargo check --target thumbv7em-none-eabihf`               |
| Размер типов = 8 байт        | unit-тест `test_size_equals_u64`                           |
| Безопасная арифметика        | `-D warnings` + отсутствие `#[allow(arithmetic_overflow)]` |
