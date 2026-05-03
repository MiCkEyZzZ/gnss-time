# Embedded Usage Guide

Как использовать `gnss-time` в окружениях `no_std` (STM32, nRF52, ESP32-C3 и т.д.).

## Quick start

```toml
# Cargo.toml
[dependencies]
gnss-time = { version = "0.5.1", default-features = false }

# For embedded logging via probe-rs:
gnss-time = { version = "0.5.1", features = ["defmt"] }
defmt      = "0.3"
```

Фича `std` не требуется. Крейт по умолчанию работает в `no_std`.

## Size guarantees

Каждый публичный тип имеет фиксированный, известный размер — подходит для
DMA-буферов и пакетов телеметрии фиксированного размера:

| Type            | Size | Alignment |
| --------------- | ---- | --------- |
| `Time<Gps>`     | 8 B  | 8 B       |
| `Time<Glonass>` | 8 B  | 8 B       |
| `Time<Galileo>` | 8 B  | 8 B       |
| `Time<Beidou>`  | 8 B  | 8 B       |
| `Time<Tai>`     | 8 B  | 8 B       |
| `Time<Utc>`     | 8 B  | 8 B       |
| `Duration`      | 8 B  | 8 B       |

Все типы-маркеры шкал (`Gps`, `Glonass`, ...) имеют нулевой размер.

## Доказательство zero-cost абстракций

Benchmark results on x86_64 (Criterion, release mode):

| Operation                                | Time    |
| ---------------------------------------- | ------- |
| `Time<Gps> + Duration` (panic-версия)    | 516 ps  |
| `u64 + u64` (базовый уровень)            | 516 ps  |
| `Time<Gps>.saturating_add`               | 516 ps  |
| `GPS → Galileo` (тождественное)          | 785 ps  |
| `GPS → TAI` (фиксированные +19 с)        | 822 ps  |
| `GPS → BeiDou` (фиксированные -14 с)     | 928 ps  |
| `GPS → UTC` (бинарный поиск, 19 записей) | 9.8 ns  |
| `UTC → GPS` (двухпроходный алгоритм)     | 22.5 ns |

Операторы `+` и `-`, вызывающие panic, компилируются ровно в те же инструкции,
что и обычная арифметика `u64` — абстракция не имеет накладных расходов во время
выполнения.

## Безопасная арифметика для embedded

В embedded-системах panic обычно означает `abort()` — без unwind.
Используйте варианты без panic:

```rust
use gnss_time::{Time, Duration, Gps};

let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
let d = Duration::from_seconds(3600);

// Option — возвращает None при переполнении
let safe: Option<Time<Gps>> = t.checked_add(d);

// Насыщение до MAX/EPOCH — никогда не паникует и не переполняется
let clamped: Time<Gps> = t.saturating_add(d);

// Возвращает GnssTimeError::Overflow при переполнении
let fallible: Result<Time<Gps>, _> = t.try_add(d);
```

## Статические инициализаторы

Ключевые типы поддерживают `const`-конструирование для использования в `static`:

```rust
use gnss_time::{Time, Duration, Gps};

static REFERENCE_EPOCH: Time<Gps> = Time::<Gps>::EPOCH;
static WINDOW: Duration = Duration::from_seconds(30);
const FIVE_MINUTES: Duration = Duration::from_seconds(300);
```

## Интеграция с defmt

Включите фичу `defmt` и добавьте `defmt = "0.3"` в зависимости:

```rust
use gnss_time::{Time, Gps};

let t = Time::<Gps>::from_week_tow(2345, 0.0).unwrap();
defmt::info!("GPS timestamp: {}", t);
// Вывод: GPS 2345:000000.000
```

Все публичные типы реализуют `defmt::Format`, если фича включена:

- `Time<S>` — тот же формат, что и `Display`
- `Duration` — формат `"Xs Yns"`
- `GnssTimeError` — короткая строка ошибки

## Кросс-компиляция

Крейт собирается для следующих embedded-таргетов (проверяется в CI):

```sh
cargo check --lib --target thumbv7em-none-eabihf        # STM32F4/F7, nRF52
cargo check --lib --target thumbv7em-none-eabi          # Cortex-M4/M7 without FPU
cargo check --lib --target riscv32imac-unknown-none-elf # ESP32-C3
```

Добавить таргеты:

```sh
rustup target add thumbv7em-none-eabihf
rustup target add riscv32imac-unknown-none-elf
```

## Парсинг пакета UBX NAV-TIMEGPS

Пример: извлечение `Time<Gps>` из бинарного пакета u-blox UBX.

```rust
use gnss_time::{GnssTimeError, Time, Gps};

/// Парсит GPS-время из payload UBX NAV-TIMEGPS (28 байт).
/// Возвращает Err, если флаги валидности времени не установлены.
pub fn parse_ubx_nav_timegps(payload: &[u8; 28]) -> Result<Time<Gps>, GnssTimeError> {
    // Байты 0..4: iTOW (миллисекунды GPS-недели)
    let itow_ms = u32::from_le_bytes(payload[0..4].try_into().unwrap()) as u64;

    // Байты 8..10: номер GPS-недели
    let week = u16::from_le_bytes(payload[8..10].try_into().unwrap());

    // Байт 24: флаги валидности. Бит 0 = towValid, бит 1 = weekValid
    let valid = payload[24];
    if valid & 0x03 != 0x03 {
        return Err(GnssTimeError::InvalidInput("UBX time not valid"));
    }

    // Перевод миллисекунд в секунды + наносекундный остаток
    let tow_s = (itow_ms / 1000) as f64;
    let frac_ms = itow_ms % 1000;
    let tow_with_ms = tow_s + frac_ms as f64 / 1000.0;

    Time::<Gps>::from_week_tow(week, tow_with_ms)
}
```

## Пример для приёмника GLONASS

```rust
use gnss_time::{GnssTimeError, Time, Glonass};

/// Парсит время GLONASS из эфемерид ICD-GLONASS.
pub fn from_glonass_icd(nt: u32, tod_s: f64) -> Result<Time<Glonass>, GnssTimeError> {
    // N_T — номер дня в 4-летнем интервале, начиная с последнего
    // 1 января високосного года. Для простоты считаем как смещение дней от эпохи.
    Time::<Glonass>::from_day_tod(nt, tod_s)
}
```

## Паттерн memory-mapped регистров

```rust
use gnss_time::{Duration, Time, Gps};

// Хранение GPS-метки времени в 64-битном регистре или ячейке FRAM.
fn write_timestamp(reg: &mut u64, t: Time<Gps>) {
    *reg = t.as_nanos();
}

fn read_timestamp(reg: u64) -> Time<Gps> {
    Time::<Gps>::from_nanos(reg)
}
```
