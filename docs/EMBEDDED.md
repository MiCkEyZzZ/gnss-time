# Embedded Usage Guide

How to use `gnss-time` in `no_std` environments (STM32, nRF52, ESP32-C3, etc.).

## Quick start

```toml
# Cargo.toml
[dependencies]
gnss-time = { version = "0.7", default-features = false }

# For embedded logging via probe-rs:
gnss-time = { version = "0.7", features = ["defmt"] }
defmt      = "0.3"

# For compact binary serialization:
gnss-time = { version = "0.7", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

The `std` feature is not required. The crate works in `no_std` by default.

## Feature flags

| Feature | Effect                                                               | Adds dependency |
| ------- | -------------------------------------------------------------------- | --------------- |
| (none)  | Pure `no_std`, no external dependencies                              | —               |
| `std`   | `impl std::error::Error` for error types                             | —               |
| `serde` | `Serialize`/`Deserialize` for `Time<S>`, `Duration`, `DurationParts` | `serde`         |
| `defmt` | `impl defmt::Format` for all public types                            | `defmt`         |

## Size guarantees

### In-memory representation

Every public type occupies **exactly 8 bytes** in memory — suitable for
DMA buffers and fixed-size telemetry packets. This refers to the in-memory
representation of the value in Rust:

| Type            | Size | Alignment |
| --------------- | ---- | --------- |
| `Time<Gps>`     | 8 B  | 8 B       |
| `Time<Glonass>` | 8 B  | 8 B       |
| `Time<Galileo>` | 8 B  | 8 B       |
| `Time<Beidou>`  | 8 B  | 8 B       |
| `Time<Tai>`     | 8 B  | 8 B       |
| `Time<Utc>`     | 8 B  | 8 B       |
| `Duration`      | 8 B  | 8 B       |

All scale marker types (`Gps`, `Glonass`, ...) are zero-sized.

> **Do not confuse in-memory and wire representations.** The in-memory size
> (8 B) is not the same as the size when serialized via `serde` + `postcard`:
> there a separate wire representation is used, described below, and its size
> **is not fixed** (it depends on the magnitude of the value).

## Proof of zero-cost abstractions

Benchmark results on x86_64 (Criterion, release mode):

| Operation                                | Time    |
| ---------------------------------------- | ------- |
| `Time<Gps> + Duration` (panicking variant) | 516 ps |
| `u64 + u64` (baseline)                   | 516 ps  |
| `Time<Gps>.saturating_add`               | 516 ps  |
| `GPS → Galileo` (identity)               | 785 ps  |
| `GPS → TAI` (fixed +19 s)                | 822 ps  |
| `GPS → BeiDou` (fixed -14 s)             | 928 ps  |
| `GPS → UTC` (binary search, 19 entries)  | 9.8 ns  |
| `UTC → GPS` (two-pass algorithm)         | 22.5 ns |

The panicking `+` and `-` operators compile down to exactly the same
instructions as plain `u64` arithmetic — the abstraction has no runtime
overhead.

## Code size (.text)

Measured automatically in CI (see `size-report` in
`.github/workflows/embedded.yml`) on the `firmware/` probe for
`thumbv7em-none-eabihf` (release). Each operation is isolated into its own
`#[inline(never)]` symbol with `black_box` guards so it can be measured
independently.

The probe deliberately avoids `unwrap()`/`panic!` and the panicking operators
(these are size probes, not a user application), so no panic/`core::fmt`
machinery ends up in `.text`. The resulting `.text` of the whole binary is
**980 B**. Most of `.text` is gnss-time code, probe functions, and the
required `cortex-m-rt` runtime infrastructure (vector table 1 KiB, `Reset`
loader 62 B, handlers ~18 B).

Measured symbols in this binary (size of a concrete ELF symbol, release):

| Symbol in this binary                            | `.text` |
| ------------------------------------------------ | ------- |
| `Time<Gps>::from_week_tow` (validation + computation) | 182 B |
| `probe_gps_to_utc` (generated function)          | 180 B   |
| `LeapSeconds::tai_minus_utc_at` (binary search)  | 138 B   |
| `Time<Gps>::to_tai` (GPS → TAI, +19 s)           | 56 B    |
| `probe_time_checked_add`                         | 56 B    |
| `probe_time_saturating_add`                      | 42 B    |
| `probe_from_week_tow` (probe wrapper)            | 34 B    |
| `probe_into_scale` (probe wrapper)               | 32 B    |

Key takeaway: `Time + Duration` requires no additional abstraction layer —
after monomorphization the operation reduces to plain arithmetic over the
internal `u64` representation. On `thumbv7em-none-eabihf` this is implemented
by a sequence of 32-bit ARM instructions (`adds`/`adcs`).
`probe_time_saturating_add` = 42 B, `probe_time_checked_add` = 56 B.

> **Panicking operators pull in the panic machinery.** The probes only check
> non-panicking operations. If you add a `+`/`-` operator (which does `panic!`
> on overflow) to the binary, the compiler also pulls in the
> `core::panic`/`core::fmt` infrastructure (~1.9 KiB: `do_count_chars`,
> `Formatter::pad`, `panic_fmt`, etc.), and the firmware `.text` grows to
> ~2.9 KiB. The panicking `+` symbol itself is only ~52 B — but the cost is in
> the panic branch. For embedded, use `saturating_add` / `checked_add` /
> `try_add`.

> **Precision of the figures.** The sizes above are sizes of concrete symbols
> in *this* binary: "generated function `probe_gps_to_utc` — 180 B", not
> "GPS → UTC costs exactly 180 B". `probe_gps_to_utc` uses the shared code of
> `into_scale_with` + `LeapSeconds::tai_minus_utc_at` (138 B) + the
> `BUILTIN_LEAP` table (8 B); part of the code may be reused by the linker
> with other symbols. The same applies to the other operations:
> `checked_add`/`saturating_add` differ at the level of these probe symbols by
> only 14 B, but that does not mean that is the full cost of the operation in
> any binary.

CI threshold: probe firmware `.text` < 2 KiB. Build and measure locally:

```sh
just setup-size   # cargo install cargo-binutils; rustup component add llvm-tools-preview
just size         # build firmware + cargo size -A + cargo bloat
```

Verify that arithmetic stayed zero-cost:

```sh
cargo objdump --release --manifest-path firmware/Cargo.toml \
  --target thumbv7em-none-eabihf -- -d \
  | grep -A8 'probe_time_saturating_add>'   # look for adds/adcs
```

> Note: `cargo bloat` answers the question "which symbols take up space",
> `cargo size -- -A` answers "how much do the `.text`/`.rodata`/... sections
> take". To check "< N bytes per operation" the only reliable source is the
> size of a concrete ELF symbol / the disassembler, because the optimizer may
> inline a function and no separate symbol remains.

## Safe arithmetic for embedded

In typical `no_std` embedded configurations panic does not use a full
unwinding runtime; the concrete behavior is determined by your
`#[panic_handler]` (for example, halting, `abort`-like behavior, or passing
panic information through `defmt`). There is no universal "panic = abort" —
for example, the probe firmware in this repository uses an infinite loop:

```rust
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

Therefore, on-device use the non-panicking variants. Example inside a function
returning `Result` (without `unwrap`):

```rust
use gnss_time::{Duration, DurationParts, Gps, GnssTimeError, Time};

fn next_window() -> Result<Time<Gps>, GnssTimeError> {
    let t = Time::<Gps>::from_week_tow(
        2345,
        DurationParts { seconds: 432_000, nanos: 0 },
    )?;

    // Option — returns None on overflow
    let safe: Option<Time<Gps>> = t.checked_add(Duration::from_seconds(3600));

    // Saturates to MAX/EPOCH — never panics
    let clamped: Time<Gps> = t.saturating_add(Duration::from_seconds(3600));

    // Returns GnssTimeError::Overflow on overflow
    let fallible: Result<Time<Gps>, GnssTimeError> = t.try_add(Duration::from_seconds(3600));

    Ok(clamped)
}
```

## Static initializers

The key types support `const` construction for use in `static`:

```rust
use gnss_time::{Time, Duration, Gps};

static REFERENCE_EPOCH: Time<Gps> = Time::<Gps>::EPOCH;
static WINDOW: Duration = Duration::from_seconds(30);
const FIVE_MINUTES: Duration = Duration::from_seconds(300);
```

## Compact binary serialization (postcard)

### Requirements

Enable the `serde` feature and add `postcard` to the dependencies:

```toml
[dependencies]
gnss-time = { version = "0.7", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

### Wire format

postcard uses **ULEB-128** (Unsigned Little-Endian Base-128) for unsigned
integers and **Zigzag + ULEB-128** for signed ones.

#### `Time<S>` — raw `u64` ULEB-128

In the compact format `Time<S>` is serialized as a raw `u64` of nanoseconds.
**The scale tag is not stored** — the scale is embedded in the Rust type
system.

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

| Value range            | Size (bytes) |
| ---------------------- | ------------ |
| 0 … 127                | 1            |
| 128 … 16 383           | 2            |
| 16 384 … 2 097 151     | 3            |
| 2 097 152 … 268 435 455| 4            |
| 268 435 456 … 2^35−1   | 5            |
| 2^35 … 2^42−1          | 6            |
| 2^42 … 2^49−1          | 7            |
| 2^49 … 2^56−1          | 8            |
| 2^56 … 2^63−1          | 9            |
| 2^63 … u64::MAX        | 10           |

> **Important:** the size is not fixed — it depends on the magnitude of the
> value. Most real GPS timestamps (~2023) require 9 bytes. Allocate a buffer
> of at least **16 bytes** for any `Time<S>`.

#### `Duration` — Zigzag + ULEB-128

`Duration` is serialized as an `i64` with Zigzag encoding (negative numbers
are encoded compactly):

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

> All byte sequences in this section are verified by golden tests
> (`serde_impls::tests::*postcard_golden` in `src/serde_impls.rs`) — they are
> the source of truth, not the other way around.

### Usage with heapless (no_std without alloc)

```rust
#![no_std]

use gnss_time::{Time, Gps, DurationParts};
use heapless::Vec;

// Serialization without alloc — stack buffer
fn serialize_gps_timestamp(t: Time<Gps>) -> Result<Vec<u8, 16>, postcard::Error> {
    postcard::to_vec(&t)
}

// Deserialization
fn deserialize_gps_timestamp(bytes: &[u8]) -> Result<Time<Gps>, postcard::Error> {
    postcard::from_bytes(bytes)
}

// Full example with a constructor
fn example() -> Result<(), postcard::Error> {
    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts { seconds: 432_000, nanos: 0 },
    )
    .unwrap();

    // Serialize into a heapless buffer (max 16 bytes)
    let buf: Vec<u8, 16> = serialize_gps_timestamp(gps)?;

    // Transmit over UART / SPI / I2C ...

    // Deserialize on the receiving side
    let decoded = deserialize_gps_timestamp(&buf)?;
    assert_eq!(gps, decoded);

    Ok(())
}
```

### Recommended buffer sizes

| Type             | Max size  | Recommended buffer |
| ---------------- | --------- | ------------------ |
| `Time<S>`        | 10 bytes  | `Vec<u8, 16>`      |
| `Duration`       | 10 bytes  | `Vec<u8, 16>`      |
| `DurationParts`  | 15 bytes  | `Vec<u8, 16>`      |
| Typical packet   | ≤ 32 bytes| `Vec<u8, 32>`      |

### Telemetry packet example

```rust
use gnss_time::{Time, Duration, Gps, DurationParts};
use heapless::Vec;

/// GPS receiver telemetry packet
#[derive(serde::Serialize, serde::Deserialize)]
struct NavPacket {
    /// GPS timestamp
    timestamp: Time<Gps>,
    /// Time correction (offset from the reference)
    clock_offset: Duration,
    /// Number of visible satellites
    sv_count: u8,
}

fn send_nav_packet(packet: &NavPacket) -> Result<Vec<u8, 32>, postcard::Error> {
    postcard::to_vec(packet)
}

fn receive_nav_packet(bytes: &[u8]) -> Result<NavPacket, postcard::Error> {
    postcard::from_bytes(bytes)
}
```

A typical packet (8 SV, 2023-year timestamp, zero correction) takes ≈ 11 bytes:

- `timestamp`: 9 bytes (ULEB-128 ~2023)
- `clock_offset`: 1 byte (zigzag(0) = 0x00)
- `sv_count`: 1 byte

### JSON ↔ postcard compatibility

The same type supports both formats. The choice is made automatically via
`is_human_readable()`:

```rust
// JSON (human-readable = true)
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

// postcard (human-readable = false)
let bytes = postcard::to_allocvec(&gps).unwrap();
// [raw ULEB-128 bytes, no scale tag]

// Both deserialize back into the same type:
let from_json: Time<Gps> = serde_json::from_str(&json).unwrap();
let from_postcard: Time<Gps> = postcard::from_bytes(&bytes).unwrap();
assert_eq!(from_json, from_postcard);
```

## defmt integration

```rust
use gnss_time::{Time, Gps, DurationParts};

let t = Time::<Gps>::from_week_tow(
    2345,
    DurationParts { seconds: 432_000, nanos: 0 },
).unwrap();
defmt::info!("GPS timestamp: {}", t);
// Output: GPS 2345:432000.000
```

All public types implement `defmt::Format` when the feature is enabled
(verified by compilation in CI: `embedded.yml` builds `--features defmt`
for every embedded target):

- `Time<S>` — the same format as `Display`
- `Duration` — format `"Xs Yns"` (same as `Display`)
- `GnssTimeError` — short error string

## Cross-compilation

Supported embedded targets (verified in CI, see `.github/workflows/embedded.yml`):

| Target                          | Architecture            | Example chips            | CI  |
| ------------------------------- | ----------------------- | -------------------------| --- |
| `thumbv7em-none-eabihf`         | Cortex-M4F/M7F + FPU    | STM32F4/F7, nRF52840     | ✅   |
| `thumbv7em-none-eabi`           | Cortex-M4/M7 without FPU| STM32F3xx                | ✅   |
| `thumbv6m-none-eabi`            | Cortex-M0/M0+           | STM32F0xx, nRF51         | ✅   |
| `riscv32imac-unknown-none-elf`  | RV32IMAC                | ESP32-C3, GD32VF103, CH32V | ✅ |
| `riscv32i-unknown-none-elf`     | RV32I (no atomics)      | ESP32-C2                 | ✅   |

For each target CI verifies the build without features and with the `defmt`
feature. A separate CI job confirms that `std` does not enter the dependency
graph transitively.

Local check:

```sh
# ARM Cortex-M
cargo check --lib --target thumbv7em-none-eabihf        # STM32F4/F7, nRF52
cargo check --lib --target thumbv7em-none-eabi          # Cortex-M4/M7 without FPU
cargo check --lib --target thumbv6m-none-eabi           # Cortex-M0/M0+

# RISC-V
cargo check --lib --target riscv32imac-unknown-none-elf # ESP32-C3
cargo check --lib --target riscv32i-unknown-none-elf    # ESP32-C2

# With serde:
cargo check --lib --features serde --target thumbv7em-none-eabihf
```

Targets are installed automatically from `rust-toolchain.toml`; or manually:

```sh
rustup target add thumbv7em-none-eabihf
rustup target add thumbv6m-none-eabi
rustup target add riscv32imac-unknown-none-elf
rustup target add riscv32i-unknown-none-elf
```

Via `just`:

```sh
just check-no-std           # thumbv7em-none-eabihf
just check-no-std-cortex-m0 # thumbv6m-none-eabi
just check-riscv            # riscv32imac + riscv32i
```

## Memory-mapped register pattern

```rust
use gnss_time::{Time, Duration, Gps};

// Storage of a GPS timestamp in a 64-bit register or FRAM cell:
fn write_timestamp(reg: &mut u64, t: Time<Gps>) {
    *reg = t.as_nanos();
}

fn read_timestamp(reg: u64) -> Time<Gps> {
    Time::<Gps>::from_nanos(reg)
}
```

## Parsing a UBX NAV-TIMEGPS packet

```rust
use gnss_time::{GnssTimeError, Time, Gps, DurationParts};

/// Parses GPS time from a UBX NAV-TIMEGPS payload (28 bytes).
pub fn parse_ubx_nav_timegps(payload: &[u8; 28]) -> Result<Time<Gps>, GnssTimeError> {
    // payload is a fixed-length array, indices 0..4 / 8..10 are statically valid
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
