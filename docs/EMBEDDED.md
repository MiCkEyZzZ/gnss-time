# Embedded Usage Guide

Как использовать `gnss-time` в окружениях `no_std` (STM32, nRF52, ESP32-C3 и т.д.).

## Quick start

```toml
# Cargo.toml
[dependencies]
gnss-time = { version = "0.6", default-features = false }

# For embedded logging via probe-rs:
gnss-time = { version = "0.6", features = ["defmt"] }
defmt      = "0.3"

# For compact binary serialization:
gnss-time = { version = "0.6", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

Фича `std` не требуется. Крейт по умолчанию работает в `no_std`.

## Feature flags

| Feature | Effect                                                               | Adds dependency |
| ------- | -------------------------------------------------------------------- | --------------- |
| (none)  | Чистый `no_std`, нет внешних зависимостей                            | —               |
| `std`   | `impl std::error::Error` для типов ошибок                            | —               |
| `serde` | `Serialize`/`Deserialize` для `Time<S>`, `Duration`, `DurationParts` | `serde`         |
| `defmt` | `impl defmt::Format` для всех публичных типов                        | `defmt`         |

## Size guarantees

### In-memory representation

Каждый публичный тип занимает в памяти **ровно 8 байт** — подходит для
DMA-буферов и пакетов телеметрии фиксированного размера. Это относится к
представлению значения в памяти Rust:

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

> **Не путайте in-memory и wire-представления.** Размер в памяти (8 B) не равен
> размеру при сериализации через `serde` + `postcard`: там используется
> отдельное wire-представление, описанное ниже, и его размер **не фиксирован**
> (зависит от величины значения).

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

## Размер кода (.text)

Измеряется автоматически в CI (см. `size-report` в `.github/workflows/embedded.yml`)
на прошивке-зонде `firmware/` для `thumbv7em-none-eabihf` (release). Каждая
операция вынесена в отдельный `#[inline(never)]`-символ с `black_box`-
ограничителями, чтобы её можно было замерить независимо.

Прошивка-зонд намеренно избегает `unwrap()`/`panic!` и panicking-операторов
(это size probe, а не пользовательское приложение), поэтому в `.text` не
попадает panic/`core::fmt`-машинерия. Итоговый `.text` всего бинарника —
**980 B**. Основная часть `.text` приходится на код `gnss-time`, probe-функции и
необходимую runtime-инфраструктуру `cortex-m-rt` (векторная таблица 1 KiB,
загрузчик `Reset` 62 B, обработчики ~18 B).

Измеренные символы в этом бинарнике (размер конкретного ELF-символа, release):

| Символ в этом бинарнике                          | `.text` |
| ------------------------------------------------ | ------- |
| `Time<Gps>::from_week_tow` (валидация + расчёт)  | 182 B   |
| `probe_gps_to_utc` (сгенерированная функция)     | 180 B   |
| `LeapSeconds::tai_minus_utc_at` (binary search)  | 138 B   |
| `Time<Gps>::to_tai` (GPS → TAI, +19 s)           | 56 B    |
| `probe_time_checked_add`                         | 56 B    |
| `probe_time_saturating_add`                      | 42 B    |
| `probe_from_week_tow` (обвязка пробы)            | 34 B    |
| `probe_into_scale` (обвязка пробы)               | 32 B    |

Ключевой вывод: `Time + Duration` не требует дополнительного слоя абстракции —
после мономорфизации операция сводится к обычной арифметике над внутренним
`u64`-представлением. На `thumbv7em-none-eabihf` это реализуется
последовательностью 32-битных ARM-инструкций (`adds`/`adcs`).
`probe_time_saturating_add` = 42 B, `probe_time_checked_add` = 56 B.

> **Panicking-операторы тянут panic-машинерию.** Пробой проверены только
> безаварийные операции. Если добавить в бинарник оператор `+`/`-` (который при
> переполнении делает `panic!`), компилятор подтягивает и
> `core::panic`/`core::fmt`-инфраструктуру (~1.9 KiB: `do_count_chars`,
> `Formatter::pad`, `panic_fmt` и т.д.), и `.text` прошивки вырастает до ~2.9 KiB.
> Сам символ panicking-`+` при этом всего ~52 B — но цена в panic-ветке.
> Для embedded используйте `saturating_add` / `checked_add` / `try_add`.

> **Точность формулировок.** Размеры выше — это размеры конкретных символов в
> *данном* бинарнике: «сгенерированная функция `probe_gps_to_utc` — 180 B», а не
> «GPS → UTC стоит ровно 180 B». `probe_gps_to_utc` использует общий код
> `into_scale_with` + `LeapSeconds::tai_minus_utc_at` (138 B) + таблицу
> `BUILTIN_LEAP` (8 B); часть кода может переиспользоваться линкером с другими
> символами. То же касается остальных операций: `checked_add`/`saturating_add`
> различаются на уровне этих probe-символов всего на 14 B, но это не значит,
> что это полная стоимость операции в любом бинарнике.

CI-порог: `.text` прошивки-зонда < 2 KiB. Сборка и замер локально:

```sh
just setup-size   # cargo install cargo-binutils; rustup component add llvm-tools-preview
just size         # build firmware + cargo size -A + cargo bloat
```

Проверка, что арифметика осталась нуль-затратной:

```sh
cargo objdump --release --manifest-path firmware/Cargo.toml \
  --target thumbv7em-none-eabihf -- -d \
  | grep -A8 'probe_time_saturating_add>'   # ищем adds/adcs
```

> Примечание: `cargo bloat` отвечает на вопрос «какие символы занимают место»,
> `cargo size -- -A` — на вопрос «сколько занимают секции `.text`/`.rodata`/...».
> Для проверки «< N байт на операцию» единственный надёжный источник — размер
> конкретного ELF-символа / дизассемблер, т.к. оптимизатор может заинлайнить
> функцию и отдельного символа не останется.

## Безопасная арифметика для embedded

В типичных `no_std` embedded-конфигурациях panic не использует полноценный
unwinding runtime; конкретное поведение определяется вашим `#[panic_handler]`
(например, остановка, `abort`-подобное поведение или передача информации о
panic через `defmt`). Универсального «panic = abort» не существует — например,
прошивка-зонд в этом репозитории использует бесконечный цикл:

```rust
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

Поэтому на борту используйте варианты без panic. Пример внутри функции,
возвращающей `Result` (без `unwrap`):

```rust
use gnss_time::{Duration, DurationParts, Gps, GnssTimeError, Time};

fn next_window() -> Result<Time<Gps>, GnssTimeError> {
    let t = Time::<Gps>::from_week_tow(
        2345,
        DurationParts { seconds: 432_000, nanos: 0 },
    )?;

    // Option — возвращает None при переполнении
    let safe: Option<Time<Gps>> = t.checked_add(Duration::from_seconds(3600));

    // Насыщение до MAX/EPOCH — никогда не паникует
    let clamped: Time<Gps> = t.saturating_add(Duration::from_seconds(3600));

    // Возвращает GnssTimeError::Overflow при переполнении
    let fallible: Result<Time<Gps>, GnssTimeError> = t.try_add(Duration::from_seconds(3600));

    Ok(clamped)
}
```

## Статические инициализаторы

Ключевые типы поддерживают `const`-конструирование для использования в `static`:

```rust
use gnss_time::{Time, Duration, Gps};

static REFERENCE_EPOCH: Time<Gps> = Time::<Gps>::EPOCH;
static WINDOW: Duration = Duration::from_seconds(30);
const FIVE_MINUTES: Duration = Duration::from_seconds(300);
```

## Compact binary serialization (postcard)

### Требования

Включите фичу `serde` и добавьте `postcard` в зависимости:

```toml
[dependencies]
gnss-time = { version = "0.6", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

### Wire format

postcard использует **ULEB-128** (Unsigned Little-Endian Base-128) для целых чисел
без знака и **Zigzag + ULEB-128** для знаковых.

#### `Time<S>` — raw `u64` ULEB-128

В компактном формате `Time<S>` сериализуется как сырое значение `u64` наносекунд.
**Тег шкалы не хранится** — шкала зашита в системе типов Rust.

```text
Encoding: ULEB-128(nanos: u64)

Examples:
  EPOCH (0 ns)                → [0x00]                    (1 byte)
  1 ns                        → [0x01]                    (1 byte)
  127 ns                      → [0x7F]                    (1 byte)
  128 ns                      → [0x80, 0x01]              (2 bytes)
  1 week (604_800_000_000_000)→ 8 bytes
  ~2023 GPS timestamp         → 9 bytes
  u64::MAX                    → [0xFF×9, 0x01]            (10 bytes)
```

| Диапазон значений       | Размер (байт) |
| ----------------------- | ------------- |
| 0 … 127                 | 1             |
| 128 … 16 383            | 2             |
| 16 384 … 2 097 151      | 3             |
| 2 097 152 … 268 435 455 | 4             |
| 268 435 456 … 2^35−1    | 5             |
| 2^35 … 2^42−1           | 6             |
| 2^42 … 2^49−1           | 7             |
| 2^49 … 2^56−1           | 8             |
| 2^56 … 2^63−1           | 9             |
| 2^63 … u64::MAX         | 10            |

> **Важно:** размер не фиксирован — он зависит от величины значения.
> Для большинства реальных GPS-меток (~2023) требуется 9 байт.
> Выделяйте буфер не менее **16 байт** для любого `Time<S>`.

#### `Duration` — Zigzag + ULEB-128

`Duration` сериализуется как `i64` с Zigzag-кодированием (отрицательные числа
кодируются компактно):

```text
Encoding: Zigzag(ULEB-128(nanos: i64))
  0  → [0x00]  (1 byte)
  -1 → [0x01]  (1 byte, zigzag maps -1 → 1)
   1 → [0x02]  (1 byte, zigzag maps  1 → 2)
```

#### `DurationParts` — tuple `[u64, u32]`

```text
Encoding: ULEB-128(seconds: u64) ++ ULEB-128(nanos: u32)

Example: { seconds: 5, nanos: 500_000_000 }
  ULEB-128(5)           → [0x05]
  ULEB-128(500_000_000) → [0x80, 0xCA, 0xB5, 0xEE, 0x01]
  Total:                → 6 bytes
```

> Все байтовые последовательности в этом разделе проверены golden-тестами
> (`serde_impls::tests::*postcard_golden` в `src/serde_impls.rs`) — они являются
> источником истины, а не наоборот.

### Использование с heapless (no_std без alloc)

```rust
#![no_std]

use gnss_time::{Time, Gps, DurationParts};
use heapless::Vec;

// Сериализация без alloc — буфер на стеке
fn serialize_gps_timestamp(t: Time<Gps>) -> Result<Vec<u8, 16>, postcard::Error> {
    postcard::to_vec(&t)
}

// Десериализация
fn deserialize_gps_timestamp(bytes: &[u8]) -> Result<Time<Gps>, postcard::Error> {
    postcard::from_bytes(bytes)
}

// Полный пример с конструктором
fn example() -> Result<(), postcard::Error> {
    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts { seconds: 432_000, nanos: 0 },
    )
    .unwrap();

    // Сериализация в heapless буфер (максимум 16 байт)
    let buf: Vec<u8, 16> = serialize_gps_timestamp(gps)?;

    // Передача по UART / SPI / I2C ...

    // Десериализация на принимающей стороне
    let decoded = deserialize_gps_timestamp(&buf)?;
    assert_eq!(gps, decoded);

    Ok(())
}
```

### Рекомендуемые размеры буферов

| Тип             | Макс. размер | Рекомендуемый буфер |
| --------------- | ------------ | ------------------- |
| `Time<S>`       | 10 байт      | `Vec<u8, 16>`       |
| `Duration`      | 10 байт      | `Vec<u8, 16>`       |
| `DurationParts` | 15 байт      | `Vec<u8, 16>`       |
| Типичный пакет  | ≤ 32 байт    | `Vec<u8, 32>`       |

### Пример телеметрического пакета

```rust
use gnss_time::{Time, Duration, Gps, DurationParts};
use heapless::Vec;

/// Телеметрический пакет GPS-приёмника
#[derive(serde::Serialize, serde::Deserialize)]
struct NavPacket {
    /// GPS-метка времени
    timestamp: Time<Gps>,
    /// Поправка к времени (отклонение от эталона)
    clock_offset: Duration,
    /// Количество видимых спутников
    sv_count: u8,
}

fn send_nav_packet(packet: &NavPacket) -> Result<Vec<u8, 32>, postcard::Error> {
    postcard::to_vec(packet)
}

fn receive_nav_packet(bytes: &[u8]) -> Result<NavPacket, postcard::Error> {
    postcard::from_bytes(bytes)
}
```

Типичный пакет (8 SV, 2023-год timestamp, нулевая поправка) занимает ≈ 11 байт:

- `timestamp`: 9 байт (ULEB-128 ~2023)
- `clock_offset`: 1 байт (zigzag(0) = 0x00)
- `sv_count`: 1 байт

### Совместимость JSON ↔ postcard

Один и тот же тип поддерживает оба формата. Выбор происходит автоматически через
`is_human_readable()`:

```rust
// JSON (human-readable = true)
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

// postcard (human-readable = false)
let bytes = postcard::to_allocvec(&gps).unwrap();
// [raw ULEB-128 bytes, no scale tag]

// Оба десериализуются обратно в тот же тип:
let from_json: Time<Gps> = serde_json::from_str(&json).unwrap();
let from_postcard: Time<Gps> = postcard::from_bytes(&bytes).unwrap();
assert_eq!(from_json, from_postcard);
```

## Интеграция с defmt

```rust
use gnss_time::{Time, Gps, DurationParts};

let t = Time::<Gps>::from_week_tow(
    2345,
    DurationParts { seconds: 432_000, nanos: 0 },
).unwrap();
defmt::info!("GPS timestamp: {}", t);
// Вывод: GPS 2345:432000.000
```

Все публичные типы реализуют `defmt::Format` при включённой фиче
(проверяется компиляцией в CI: `embedded.yml` собирает `--features defmt`
для каждого embedded-таргета):

- `Time<S>` — тот же формат, что и `Display`
- `Duration` — формат `"Xs Yns"` (как `Display`)
- `GnssTimeError` — короткая строка ошибки

## Кросс-компиляция

Поддерживаемые embedded-таргеты (проверяются в CI, см. `.github/workflows/embedded.yml`):

| Target                          | Архитектура            | Примеры чипов              | CI  |
| ------------------------------- | ---------------------- | ---------------------------| --- |
| `thumbv7em-none-eabihf`         | Cortex-M4F/M7F + FPU   | STM32F4/F7, nRF52840       | ✅   |
| `thumbv7em-none-eabi`           | Cortex-M4/M7 без FPU   | STM32F3xx                  | ✅   |
| `thumbv6m-none-eabi`            | Cortex-M0/M0+          | STM32F0xx, nRF51           | ✅   |
| `riscv32imac-unknown-none-elf`  | RV32IMAC               | ESP32-C3, GD32VF103, CH32V | ✅   |
| `riscv32i-unknown-none-elf`     | RV32I (без атомиков)   | ESP32-C2                   | ✅   |

Для каждого таргета CI проверяет сборку без фич и с фичей `defmt`. Отдельная
CI-джоба подтверждает, что `std` не попадает в граф зависимостей транзитивно.

Локальная проверка:

```sh
# ARM Cortex-M
cargo check --lib --target thumbv7em-none-eabihf        # STM32F4/F7, nRF52
cargo check --lib --target thumbv7em-none-eabi          # Cortex-M4/M7 без FPU
cargo check --lib --target thumbv6m-none-eabi           # Cortex-M0/M0+

# RISC-V
cargo check --lib --target riscv32imac-unknown-none-elf # ESP32-C3
cargo check --lib --target riscv32i-unknown-none-elf    # ESP32-C2

# С serde:
cargo check --lib --features serde --target thumbv7em-none-eabihf
```

Таргеты устанавливаются автоматически из `rust-toolchain.toml`; либо вручную:

```sh
rustup target add thumbv7em-none-eabihf
rustup target add thumbv6m-none-eabi
rustup target add riscv32imac-unknown-none-elf
rustup target add riscv32i-unknown-none-elf
```

Через `just`:

```sh
just check-no-std           # thumbv7em-none-eabihf
just check-no-std-cortex-m0 # thumbv6m-none-eabi
just check-riscv            # riscv32imac + riscv32i
```

## Паттерн memory-mapped регистров

```rust
use gnss_time::{Time, Duration, Gps};

// Хранение GPS-метки в 64-битном регистре или ячейке FRAM:
fn write_timestamp(reg: &mut u64, t: Time<Gps>) {
    *reg = t.as_nanos();
}

fn read_timestamp(reg: u64) -> Time<Gps> {
    Time::<Gps>::from_nanos(reg)
}
```

## Парсинг пакета UBX NAV-TIMEGPS

```rust
use gnss_time::{GnssTimeError, Time, Gps, DurationParts};

/// Парсит GPS-время из payload UBX NAV-TIMEGPS (28 байт).
pub fn parse_ubx_nav_timegps(payload: &[u8; 28]) -> Result<Time<Gps>, GnssTimeError> {
    // payload — массив фиксированной длины, индексы 0..4 / 8..10 статически валидны
    let itow_ms = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]) as u64;
    let week    = u16::from_le_bytes([payload[8], payload[9]]);
    let valid   = payload[24];

    if valid & 0x03 != 0x03 {
        return Err(GnssTimeError::InvalidInput("UBX time not valid"));
    }

    let tow_s     = itow_ms / 1000;
    let tow_ms_r  = itow_ms % 1000;

    Time::<Gps>::from_week_tow(
        week,
        DurationParts { seconds: tow_s, nanos: (tow_ms_r * 1_000_000) as u32 },
    )
}
```
