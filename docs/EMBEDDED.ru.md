# Руководство по использованию во встраиваемых системах

В этом руководстве описано использование `gnss-time` в средах `no_std` (STM32, nRF52, ESP32-C3 и т. д.).

## Быстрый старт

```toml
# Cargo.toml
[dependencies]
gnss-time = { version = "0.8", default-features = false }

# Для встроенного логирования через probe-rs:
gnss-time = { version = "0.8", features = ["defmt"] }
defmt      = "0.3"

# Для компактной бинарной сериализации:
gnss-time = { version = "0.8", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

Флаг `std` не требуется. Крейт по умолчанию работает в `no_std`.

## Feature-флаги

| Feature | Эффект                                                               | Добавляет зависимость |
| ------- | -------------------------------------------------------------------- | ---------------------- |
| (нет)   | Чистый `no_std`, без внешних зависимостей                            | —                      |
| `std`   | `impl std::error::Error` для типов ошибок                            | —                      |
| `serde` | `Serialize`/`Deserialize` для `Time<S>`, `Duration`, `DurationParts` | `serde`                |
| `defmt` | `impl defmt::Format` для всех публичных типов                        | `defmt`                |

## Гарантии размера

### Представление в памяти

Каждый из перечисленных ниже типов времени и длительности занимает в памяти
**ровно 8 байт** — подходит для DMA-буферов и телеметрических пакетов
фиксированного размера. Это относится к представлению значения в памяти в Rust:

| Тип             | Размер | Выравнивание |
| --------------- | ------ | ------------ |
| `Time<Gps>`     | 8 Б    | 8 Б          |
| `Time<Glonass>` | 8 Б    | 8 Б          |
| `Time<Galileo>` | 8 Б    | 8 Б          |
| `Time<Beidou>`  | 8 Б    | 8 Б          |
| `Time<Tai>`     | 8 Б    | 8 Б          |
| `Time<Utc>`     | 8 Б    | 8 Б          |
| `Duration`      | 8 Б    | 8 Б          |

Все маркерные типы шкал (`Gps`, `Glonass`, …) имеют нулевой размер (zero-sized).

> **Не путайте представление в памяти и wire-представление.** Размер в памяти
> (8 Б) — это не то же самое, что размер при сериализации через `serde` +
> `postcard`: там используется отдельное wire-представление, описанное ниже,
> и его размер **не зафиксирован** (он зависит от величины значения).

## Доказательство нулевой стоимости абстракций

Результаты бенчмарков на x86_64 (Criterion, release-режим):

| Операция                                         | Время   |
| ------------------------------------------------ | ------- |
| `Time<Gps> + Duration` (паника при переполнении) | 516 ps  |
| `u64 + u64` (базовый уровень)                    | 516 ps  |
| `Time<Gps>.saturating_add`                       | 516 ps  |
| `GPS → Galileo` (тождественное)                  | 785 ps  |
| `GPS → TAI` (фикс. +19 с)                        | 822 ps  |
| `GPS → BeiDou` (фикс. −14 с)                     | 928 ps  |
| `GPS → UTC` (двоичный поиск, 19 записей)         | 9,8 ns  |
| `UTC → GPS` (двухпроходный алгоритм)             | 22,5 ns |

Операторы `+` и `-` вызывают панику при переполнении; после мономорфизации
они сводятся к той же элементарной арифметике, что и для базового значения
`u64`, — у абстракции нет затрат во время выполнения.

## Размер кода (.text)

Измеряется автоматически в CI (см. `size-report` в
`.github/workflows/embedded.yml`) на прошивке-пробнике (`firmware/`) для
`thumbv7em-none-eabihf` (release). Каждая операция изолирована в отдельный
символ с `#[inline(never)]` и защищена `black_box`, чтобы её можно было
измерять независимо.

Пробник намеренно избегает `unwrap()`/`panic!` и паникующих операторов (это
зонды размера, а не пользовательское приложение), поэтому в `.text` не
попадает паникующая/`core::fmt`-инфраструктура. Итоговый `.text` всего
бинарника составляет **980 Б**. Большую часть `.text` составляют код
gnss-time, функции-пробники и необходимая инфраструктура рантайма
`cortex-m-rt` (таблица векторов 1 КиБ, загрузчик `Reset` 62 Б, обработчики
~18 Б).

Измеренные символы в этом бинарнике (размер конкретного ELF-символа, release):

| Символ в этом бинарнике                                  | `.text` |
| -------------------------------------------------------- | ------- |
| `Time<Gps>::from_week_tow` (проверки + вычисление)       | 182 Б   |
| `probe_gps_to_utc` (сгенерированная функция)             | 180 Б   |
| `LeapSeconds::tai_minus_utc_at` (двоичный поиск)         | 138 Б   |
| `Time<Gps>::to_tai` (GPS → TAI, +19 с)                   | 56 Б    |
| `probe_time_checked_add`                                 | 56 Б    |
| `probe_time_saturating_add`                              | 42 Б    |
| `probe_from_week_tow` (обёртка-пробник)                  | 34 Б    |
| `probe_into_scale` (обёртка-пробник)                     | 32 Б    |

Ключевой вывод: для `Time + Duration` не нужен дополнительный уровень
абстракции — после мономорфизации операция сводится к простой арифметике над
внутренним представлением `u64`. На `thumbv7em-none-eabihf` это реализуется
последовательностью 32-битных ARM-инструкций (`adds`/`adcs`).
`probe_time_saturating_add` = 42 Б, `probe_time_checked_add` = 56 Б.

> **Паникующие операторы тянут за собой паникующую инфраструктуру.** Пробники
> проверяют только не паникующие операции. Если добавить в бинарник оператор
> `+`/`-` (который вызывает `panic!` при переполнении), компилятор также
> подтянет инфраструктуру `core::panic`/`core::fmt` (~1,9 КиБ: `do_count_chars`,
> `Formatter::pad`, `panic_fmt` и т. д.), и `.text` прошивки вырастет до
> ~2,9 КиБ. Сам паникующий символ `+` занимает лишь ~52 Б — но цена кроется в
> ветке паники. Для встраиваемых систем используйте `saturating_add` /
> `checked_add` / `try_add`.

> **Точность цифр.** Указанные выше размеры — это размеры конкретных символов
> в *этом* бинарнике: «сгенерированная функция `probe_gps_to_utc` — 180 Б», а
> не «преобразование GPS → UTC стоит ровно 180 Б». `probe_gps_to_utc`
> использует общий код `into_scale_with` + `LeapSeconds::tai_minus_utc_at`
> (138 Б) + таблицу `BUILTIN_LEAP` (8 Б); часть кода может переиспользоваться
> линкером вместе с другими символами. То же относится и к остальным
> операциям: `checked_add`/`saturating_add` различаются на уровне этих
> пробных символов всего на 14 Б, но это не значит, что такова полная
> стоимость операции в произвольном бинарнике.

Порог CI: `.text` прошивки-пробника < 2 КиБ. Сборка и измерение локально:

```sh
just setup-size   # cargo install cargo-binutils; rustup component add llvm-tools-preview
just size         # build firmware + cargo size -A + cargo bloat
```

Проверка, что арифметика осталась zero-cost:

```sh
cargo objdump --release --manifest-path firmware/Cargo.toml \
  --target thumbv7em-none-eabihf -- -d \
  | grep -A8 'probe_time_saturating_add>'   # look for adds/adcs
```

> Примечание: `cargo bloat` отвечает на вопрос «какие символы занимают место»,
> а `cargo size -- -A` — на вопрос «сколько занимают секции
> `.text`/`.rodata`/...». Чтобы проверить «< N байт на операцию», единственный
> надёжный источник — размер конкретного ELF-символа либо дизассемблер,
> потому что оптимизатор может встроить функцию, и отдельного символа не
> останется.

## Безопасная арифметика для встраиваемых систем

В типичных встраиваемых конфигурациях `no_std` паника не использует
полноценный рантайм размотки (unwinding); конкретное поведение определяет ваш
`#[panic_handler]` (например, остановка, поведение в духе `abort` или передача
информации о панике через `defmt`). Универсального правила «паника = abort»
не существует — например, прошивка-пробник в этом репозитории использует
бесконечный цикл:

```rust
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

Поэтому на устройстве используйте не паникующие варианты. Пример внутри
функции, возвращающей `Result` (без `unwrap`):

```rust
use gnss_time::{Duration, DurationParts, Gps, GnssTimeError, Time};

fn next_window() -> Result<Time<Gps>, GnssTimeError> {
    let t = Time::<Gps>::from_week_tow(
        2345,
        DurationParts { seconds: 432_000, nanos: 0 },
    )?;

    // Option — возвращает None при переполнении
    let safe: Option<Time<Gps>> = t.checked_add(Duration::from_seconds(3600));

    // Насыщает до MAX/EPOCH — никогда не паникует
    let clamped: Time<Gps> = t.saturating_add(Duration::from_seconds(3600));

    // Возвращает GnssTimeError::Overflow при переполнении
    let fallible: Result<Time<Gps>, GnssTimeError> = t.try_add(Duration::from_seconds(3600));

    Ok(clamped)
}
```

## Статическая инициализация

Ключевые типы поддерживают конструирование в `const` для использования в `static`:

```rust
use gnss_time::{Time, Duration, Gps};

static REFERENCE_EPOCH: Time<Gps> = Time::<Gps>::EPOCH;
static WINDOW: Duration = Duration::from_seconds(30);
const FIVE_MINUTES: Duration = Duration::from_seconds(300);
```

## Компактная бинарная сериализация (postcard)

### Требования

Включите флаг `serde` и добавьте `postcard` в зависимости:

```toml
[dependencies]
gnss-time = { version = "0.8", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

### Wire-формат

postcard использует **ULEB-128** (Unsigned Little-Endian Base-128) для
беззнаковых целых и **Zigzag + ULEB-128** — для знаковых.

#### `Time<S>` — сырое `u64`, ULEB-128

В компактном формате `Time<S>` сериализуется как сырое значение `u64` в
наносекундах. **Тег шкалы не сохраняется** — шкала встроена в систему типов
Rust.

```text
Кодирование: ULEB-128(nanos: u64)

Примеры:
  EPOCH (0 нс)                   → [0x00]                    (1 байт)
  1 нс                           → [0x01]                    (1 байт)
  127 нс                         → [0x7F]                    (1 байт)
  128 нс                         → [0x80, 0x01]              (2 байта)
  1 неделя (604_800_000_000_000) → 8 байт
  GPS-метка времени ~2023        → 9 байт
  u64::MAX                       → [0xFF×9, 0x01]            (10 байт)
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
> Большинство реальных GPS-меток времени (~2023) требуют 9 байт.
> Резервируйте буфер не менее **16 байт** для любого `Time<S>`.

#### `Duration` — Zigzag + ULEB-128

`Duration` сериализуется как `i64` с кодировкой Zigzag (отрицательные числа
кодируются компактно):

```text
Кодирование: Zigzag(ULEB-128(nanos: i64))
  0  → [0x00]  (1 байт)
  -1 → [0x01]  (1 байт, zigzag отображает -1 → 1)
   1 → [0x02]  (1 байт, zigzag отображает  1 → 2)
```

#### `DurationParts` — кортеж `[u64, u32]`

```text
Кодирование: ULEB-128(seconds: u64) ++ ULEB-128(nanos: u32)

Пример: { seconds: 5, nanos: 500_000_000 }
  ULEB-128(5)           → [0x05]
  ULEB-128(500_000_000) → [0x80, 0xCA, 0xB5, 0xEE, 0x01]
  Итого:                → 6 байт
```

> Все байтовые последовательности в этом разделе проверяются golden-тестами
> (`serde_impls::tests::*postcard_golden` в `src/serde_impls.rs`) — именно они
> являются источником истины, а не наоборот.

### Использование с heapless (no_std без alloc)

```rust
#![no_std]

use gnss_time::{Time, Gps, DurationParts};
use heapless::Vec;

// Сериализация без alloc — стековый буфер
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

    // Сериализация в heapless-буфер (макс. 16 байт)
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
    /// Коррекция времени (смещение от опорного значения)
    clock_offset: Duration,
    /// Число видимых спутников
    sv_count: u8,
}

fn send_nav_packet(packet: &NavPacket) -> Result<Vec<u8, 32>, postcard::Error> {
    postcard::to_vec(packet)
}

fn receive_nav_packet(bytes: &[u8]) -> Result<NavPacket, postcard::Error> {
    postcard::from_bytes(bytes)
}
```

Типичный пакет (8 SV, метка времени из ~2023, нулевая коррекция) занимает ≈ 11 байт:

- `timestamp`: 9 байт (ULEB-128 ~2023)
- `clock_offset`: 1 байт (zigzag(0) = 0x00)
- `sv_count`: 1 байт

### Совместимость JSON ↔ postcard

Один и тот же тип поддерживает оба формата. Выбор делается автоматически
через `is_human_readable()`:

```rust
// JSON (human-readable = true)
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

// postcard (human-readable = false)
let bytes = postcard::to_allocvec(&gps).unwrap();
// [сырые байты ULEB-128, без тега шкалы]

// Оба формата десериализуются обратно в тот же тип:
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

Все публичные типы реализуют `defmt::Format`, когда флаг включён (проверяется
компиляцией в CI: `embedded.yml` собирает `--features defmt` для каждой
встраиваемой цели):

- `Time<S>` — тот же формат, что и `Display`
- `Duration` — формат `"Xs Yns"` (тот же, что и `Display`)
- `GnssTimeError` — короткая строка ошибки

## Кросс-компиляция

Поддерживаемые встраиваемые цели (проверено в CI, см. `.github/workflows/embedded.yml`):

| Цель                            | Архитектура              | Примеры чипов              | CI  |
| ------------------------------- | ------------------------ | -------------------------- | --- |
| `thumbv7em-none-eabihf`         | Cortex-M4F/M7F + FPU     | STM32F4/F7, nRF52840       | ✅  |
| `thumbv7em-none-eabi`           | Cortex-M4/M7 без FPU     | STM32F3xx                  | ✅  |
| `thumbv6m-none-eabi`            | Cortex-M0/M0+            | STM32F0xx, nRF51           | ✅  |
| `riscv32imac-unknown-none-elf`  | RV32IMAC                 | ESP32-C3, GD32VF103, CH32V | ✅  |
| `riscv32i-unknown-none-elf`     | RV32I (без атомарных операций) | ESP32-C2            | ✅   |

Для каждой цели CI проверяет сборку без флагов и с флагом `defmt`. Отдельная
CI-задача подтверждает, что `std` не попадает в граф зависимостей транзитивно.

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

Цели устанавливаются автоматически из `rust-toolchain.toml`; либо вручную:

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

// Сохранение GPS-метки времени в 64-битном регистре или ячейке FRAM:
fn write_timestamp(reg: &mut u64, t: Time<Gps>) {
    *reg = t.as_nanos();
}

fn read_timestamp(reg: u64) -> Time<Gps> {
    Time::<Gps>::from_nanos(reg)
}
```

## Разбор пакета UBX NAV-TIMEGPS

```rust
use gnss_time::{GnssTimeError, Time, Gps, DurationParts};

/// Разбирает GPS-время из полезной нагрузки UBX NAV-TIMEGPS (28 байт).
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
