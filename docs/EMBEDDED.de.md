# Anleitung für den Einsatz in Embedded-Systemen

Diese Anleitung beschreibt die Verwendung von `gnss-time` in `no_std`-Umgebungen (STM32, nRF52, ESP32-C3 usw.).

## Schnellstart

```toml
# Cargo.toml
[dependencies]
gnss-time = { version = "0.8", default-features = false }

# Für Embedded-Logging über probe-rs:
gnss-time = { version = "0.8", features = ["defmt"] }
defmt      = "0.3"

# Für kompakte Binärserialisierung:
gnss-time = { version = "0.8", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

Das `std`-Feature ist nicht erforderlich. Das Crate funktioniert standardmäßig in `no_std`.

## Feature-Flags

| Feature | Wirkung                                                              | Zusätzliche Abhängigkeit |
| ------- | -------------------------------------------------------------------- | ------------------------ |
| (keine) | Reines `no_std`, keine externen Abhängigkeiten                       | —                        |
| `std`   | `impl std::error::Error` für Fehlertypen                             | —                        |
| `serde` | `Serialize`/`Deserialize` für `Time<S>`, `Duration`, `DurationParts` | `serde`                  |
| `defmt` | `impl defmt::Format` für alle öffentlichen Typen                     | `defmt`                  |

## Größengarantien

### Darstellung im Speicher

Jeder der unten aufgeführten Zeit- und Dauer-Typen belegt im Speicher
**exakt 8 Bytes** — geeignet für DMA-Puffer und Telemetriepakete mit fester
Größe. Das bezieht sich auf die In-Memory-Darstellung des Werts in Rust:

| Typ             | Größe | Ausrichtung |
| --------------- | ----- | ----------- |
| `Time<Gps>`     | 8 B   | 8 B         |
| `Time<Glonass>` | 8 B   | 8 B         |
| `Time<Galileo>` | 8 B   | 8 B         |
| `Time<Beidou>`  | 8 B   | 8 B         |
| `Time<Tai>`     | 8 B   | 8 B         |
| `Time<Utc>`     | 8 B   | 8 B         |
| `Duration`      | 8 B   | 8 B         |

Alle Skalen-Markertypen (`Gps`, `Glonass`, …) sind Zero-Sized Types.

> **In-Memory- und Wire-Darstellung nicht verwechseln.** Die Größe im Speicher
> (8 B) ist nicht identisch mit der Größe bei der Serialisierung über `serde` +
> `postcard`: dort kommt eine separate Wire-Darstellung zum Einsatz, die weiter
> unten beschrieben wird und deren Größe **nicht fest** ist (sie hängt von der
> Größenordnung des Werts ab).

## Nachweis der Zero-Cost-Abstraktionen

Bench-Ergebnisse auf x86_64 (Criterion, Release-Modus):

| Operation                                    | Zeit   |
| -------------------------------------------- | ------ |
| `Time<Gps> + Duration` (Panik bei Überlauf)        | 516 ps |
| `u64 + u64` (Basiswert)                      | 516 ps |
| `Time<Gps>.saturating_add`                   | 516 ps |
| `GPS → Galileo` (Identität)                  | 785 ps |
| `GPS → TAI` (fix +19 s)                      | 822 ps |
| `GPS → BeiDou` (fix −14 s)                   | 928 ps |
| `GPS → UTC` (binäre Suche, 19 Einträge)      | 9,8 ns |
| `UTC → GPS` (Zwei-Pass-Algorithmus)          | 22,5 ns |

Die Operatoren `+` und `-` lösen bei einem Überlauf eine Panik aus; nach der
Monomorphisierung reduzieren sie sich auf dieselbe elementare Arithmetik wie
der zugrunde liegende `u64`-Wert — die Abstraktion hat keinen
Laufzeit-Overhead.

## Codegröße (.text)

Wird automatisch in CI gemessen (siehe `size-report` in
`.github/workflows/embedded.yml`) an der `firmware/`-Probe für
`thumbv7em-none-eabihf` (Release). Jede Operation ist in ein eigenes,
`#[inline(never)]`-Symbol mit `black_box`-Guards isoliert, damit sie
unabhängig gemessen werden kann.

Die Probe verzichtet bewusst auf `unwrap()`/`panic!` und die
Panik-Operatoren (es handelt sich um Größen-Sonden, nicht um eine
Anwenderanwendung), sodass keine Panik-/`core::fmt`-Infrastruktur im `.text`
landet. Der resultierende `.text` des gesamten Binaries beträgt **980 B**. Der
größte Teil des `.text` besteht aus gnss-time-Code, Probe-Funktionen und der
erforderlichen `cortex-m-rt`-Laufzeitinfrastruktur (Vektortabelle 1 KiB,
`Reset`-Loader 62 B, Handler ~18 B).

Gemessene Symbole in diesem Binary (Größe eines konkreten ELF-Symbols, Release):

| Symbol in diesem Binary                     | `.text` |
| ------------------------------------------- | ------- |
| `Time<Gps>::from_week_tow` (Validierung + Berechnung) | 182 B |
| `probe_gps_to_utc` (generierte Funktion)    | 180 B   |
| `LeapSeconds::tai_minus_utc_at` (binäre Suche) | 138 B |
| `Time<Gps>::to_tai` (GPS → TAI, +19 s)      | 56 B    |
| `probe_time_checked_add`                    | 56 B    |
| `probe_time_saturating_add`                 | 42 B    |
| `probe_from_week_tow` (Probe-Wrapper)       | 34 B    |
| `probe_into_scale` (Probe-Wrapper)          | 32 B    |

Kernaussage: `Time + Duration` benötigt keine zusätzliche
Abstraktionsebene — nach der Monomorphisierung reduziert sich die Operation
auf einfache Arithmetik über der internen `u64`-Darstellung. Auf
`thumbv7em-none-eabihf` wird dies durch eine Sequenz von 32-Bit-ARM-Instruktionen
(`adds`/`adcs`) umgesetzt. `probe_time_saturating_add` = 42 B,
`probe_time_checked_add` = 56 B.

> **Panik-Operatoren ziehen die Panik-Infrastruktur mit ein.** Die Sonden
> prüfen nur nicht-panikartige Operationen. Wenn Sie einen `+`/`-`-Operator
> (der bei Überlauf `panic!` auslöst) in das Binary aufnehmen, zieht der
> Compiler auch die `core::panic`/`core::fmt`-Infrastruktur mit ein
> (~1,9 KiB: `do_count_chars`, `Formatter::pad`, `panic_fmt` usw.), und der
> Firmware-`.text` wächst auf ~2,9 KiB. Das panikauslösende `+`-Symbol selbst
> ist nur ~52 B groß — aber der Preis steckt im Panik-Zweig. Verwenden Sie für
> Embedded `saturating_add` / `checked_add` / `try_add`.

> **Genauigkeit der Zahlenangaben.** Bei den obigen Größen handelt es sich um
> die Größen konkreter Symbole in *diesem* Binary: „generierte Funktion
> `probe_gps_to_utc` — 180 B“, nicht „GPS → UTC kostet exakt 180 B“.
> `probe_gps_to_utc` verwendet den gemeinsamen Code von `into_scale_with` +
> `LeapSeconds::tai_minus_utc_at` (138 B) + die `BUILTIN_LEAP`-Tabelle (8 B);
> ein Teil des Codes kann vom Linker mit anderen Symbolen gemeinsam genutzt
> werden. Dasselbe gilt für die übrigen Operationen: `checked_add`/
> `saturating_add` unterscheiden sich auf Ebene dieser Probe-Symbole nur um
> 14 B, aber das bedeutet nicht, dass das die vollständigen Kosten der
> Operation in einem beliebigen Binary wären.

CI-Schwellenwert: `.text` der Probe-Firmware < 2 KiB. Lokal bauen und messen:

```sh
just setup-size   # cargo install cargo-binutils; rustup component add llvm-tools-preview
just size         # build firmware + cargo size -A + cargo bloat
```

Verifizieren, dass die Arithmetik zero-cost geblieben ist:

```sh
cargo objdump --release --manifest-path firmware/Cargo.toml \
  --target thumbv7em-none-eabihf -- -d \
  | grep -A8 'probe_time_saturating_add>'   # look for adds/adcs
```

> Hinweis: `cargo bloat` beantwortet die Frage „welche Symbole belegen
> Speicherplatz“, `cargo size -- -A` die Frage „wie viel nehmen die
> `.text`/`.rodata`/...-Sektionen ein“. Um „< N Bytes pro Operation“ zu
> prüfen, ist die einzige verlässliche Quelle die Größe eines konkreten
> ELF-Symbols bzw. der Disassembler, denn der Optimierer kann eine Funktion
> einbinden (inlinen), sodass kein separates Symbol mehr übrig bleibt.

## Sichere Arithmetik für Embedded

In typischen `no_std`-Embedded-Konfigurationen nutzt Panik keine vollständige
Unwind-Laufzeit; das konkrete Verhalten bestimmt Ihr `#[panic_handler]`
(zum Beispiel Anhalten, `abort`-artiges Verhalten oder die Übergabe der
Panik-Information über `defmt`). Es gibt kein universelles „Panik = abort“ —
die Probe-Firmware in diesem Repository verwendet beispielsweise eine
Endlosschleife:

```rust
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

Verwenden Sie daher auf dem Gerät die nicht-panikartigen Varianten. Beispiel
innerhalb einer Funktion, die `Result` zurückgibt (ohne `unwrap`):

```rust
use gnss_time::{Duration, DurationParts, Gps, GnssTimeError, Time};

fn next_window() -> Result<Time<Gps>, GnssTimeError> {
    let t = Time::<Gps>::from_week_tow(
        2345,
        DurationParts { seconds: 432_000, nanos: 0 },
    )?;

    // Option — gibt bei Überlauf None zurück
    let safe: Option<Time<Gps>> = t.checked_add(Duration::from_seconds(3600));

    // Saturiert zu MAX/EPOCH — löst nie eine Panik aus
    let clamped: Time<Gps> = t.saturating_add(Duration::from_seconds(3600));

    // Gibt bei Überlauf GnssTimeError::Overflow zurück
    let fallible: Result<Time<Gps>, GnssTimeError> = t.try_add(Duration::from_seconds(3600));

    Ok(clamped)
}
```

## Statische Initialisierung

Die zentralen Typen unterstützen `const`-Konstruktion für den Einsatz in `static`:

```rust
use gnss_time::{Time, Duration, Gps};

static REFERENCE_EPOCH: Time<Gps> = Time::<Gps>::EPOCH;
static WINDOW: Duration = Duration::from_seconds(30);
const FIVE_MINUTES: Duration = Duration::from_seconds(300);
```

## Kompakte Binärserialisierung (postcard)

### Voraussetzungen

Aktivieren Sie das `serde`-Feature und fügen Sie `postcard` zu den
Abhängigkeiten hinzu:

```toml
[dependencies]
gnss-time = { version = "0.8", features = ["serde"] }
postcard   = { version = "1", default-features = false, features = ["heapless"] }
heapless   = "0.8"
serde      = { version = "1", default-features = false }
```

### Wire-Format

postcard verwendet **ULEB-128** (Unsigned Little-Endian Base-128) für
vorzeichenlose Ganzzahlen und **Zigzag + ULEB-128** für vorzeichenbehaftete.

#### `Time<S>` — rohes `u64` als ULEB-128

Im kompakten Format wird `Time<S>` als roher `u64`-Nanosekundenwert
serialisiert. **Der Skalen-Tag wird nicht gespeichert** — die Skala ist im
Rust-Typsystem eingebettet.

```text
Kodierung: ULEB-128(nanos: u64)

Beispiele:
  EPOCH (0 ns)                  → [0x00]                    (1 Byte)
  1 ns                          → [0x01]                    (1 Byte)
  127 ns                        → [0x7F]                    (1 Byte)
  128 ns                        → [0x80, 0x01]              (2 Bytes)
  1 Woche (604_800_000_000_000) → 8 Bytes
  ~2023-GPS-Zeitstempel         → 9 Bytes
  u64::MAX                      → [0xFF×9, 0x01]            (10 Bytes)
```

| Wertebereich           | Größe (Bytes) |
| ---------------------- | ------------- |
| 0 … 127                | 1             |
| 128 … 16 383           | 2             |
| 16 384 … 2 097 151     | 3             |
| 2 097 152 … 268 435 455| 4             |
| 268 435 456 … 2^35−1   | 5             |
| 2^35 … 2^42−1          | 6             |
| 2^42 … 2^49−1          | 7             |
| 2^49 … 2^56−1          | 8             |
| 2^56 … 2^63−1          | 9             |
| 2^63 … u64::MAX        | 10            |

> **Wichtig:** Die Größe ist nicht fest — sie hängt von der Größenordnung des
> Werts ab. Die meisten realen GPS-Zeitstempel (~2023) benötigen 9 Bytes.
> Reservieren Sie für jedes `Time<S>` einen Puffer von mindestens **16 Bytes**.

#### `Duration` — Zigzag + ULEB-128

`Duration` wird als `i64` mit Zigzag-Kodierung serialisiert (negative Zahlen
werden kompakt kodiert):

```text
Kodierung: Zigzag(ULEB-128(nanos: i64))
  0  → [0x00]  (1 Byte)
  -1 → [0x01]  (1 Byte, Zigzag bildet -1 → 1 ab)
   1 → [0x02]  (1 Byte, Zigzag bildet  1 → 2 ab)
```

#### `DurationParts` — Tupel `[u64, u32]`

```text
Kodierung: ULEB-128(seconds: u64) ++ ULEB-128(nanos: u32)

Beispiel: { seconds: 5, nanos: 500_000_000 }
  ULEB-128(5)           → [0x05]
  ULEB-128(500_000_000) → [0x80, 0xCA, 0xB5, 0xEE, 0x01]
  Gesamt:               → 6 Bytes
```

> Alle Byte-Sequenzen in diesem Abschnitt werden durch Golden Tests
> verifiziert (`serde_impls::tests::*postcard_golden` in `src/serde_impls.rs`)
> — sie sind die Quelle der Wahrheit, nicht umgekehrt.

### Verwendung mit heapless (no_std ohne alloc)

```rust
#![no_std]

use gnss_time::{Time, Gps, DurationParts};
use heapless::Vec;

// Serialisierung ohne alloc — Stack-Puffer
fn serialize_gps_timestamp(t: Time<Gps>) -> Result<Vec<u8, 16>, postcard::Error> {
    postcard::to_vec(&t)
}

// Deserialisierung
fn deserialize_gps_timestamp(bytes: &[u8]) -> Result<Time<Gps>, postcard::Error> {
    postcard::from_bytes(bytes)
}

// Vollständiges Beispiel mit einem Konstruktor
fn example() -> Result<(), postcard::Error> {
    let gps = Time::<Gps>::from_week_tow(
        2345,
        DurationParts { seconds: 432_000, nanos: 0 },
    )
    .unwrap();

    // In einen heapless-Puffer serialisieren (max. 16 Bytes)
    let buf: Vec<u8, 16> = serialize_gps_timestamp(gps)?;

    // Über UART / SPI / I2C übertragen ...

    // Auf der Empfängerseite deserialisieren
    let decoded = deserialize_gps_timestamp(&buf)?;
    assert_eq!(gps, decoded);

    Ok(())
}
```

### Empfohlene Puffergrößen

| Typ             | Max. Größe | Empfohlener Puffer |
| --------------- | ---------- | ------------------ |
| `Time<S>`       | 10 Bytes   | `Vec<u8, 16>`      |
| `Duration`      | 10 Bytes   | `Vec<u8, 16>`      |
| `DurationParts` | 15 Bytes   | `Vec<u8, 16>`      |
| Typisches Paket | ≤ 32 Bytes | `Vec<u8, 32>`      |

### Beispiel für ein Telemetriepaket

```rust
use gnss_time::{Time, Duration, Gps, DurationParts};
use heapless::Vec;

/// Telemetriepaket des GPS-Empfängers
#[derive(serde::Serialize, serde::Deserialize)]
struct NavPacket {
    /// GPS-Zeitstempel
    timestamp: Time<Gps>,
    /// Zeitkorrektur (Abweichung von der Referenz)
    clock_offset: Duration,
    /// Anzahl sichtbarer Satelliten
    sv_count: u8,
}

fn send_nav_packet(packet: &NavPacket) -> Result<Vec<u8, 32>, postcard::Error> {
    postcard::to_vec(packet)
}

fn receive_nav_packet(bytes: &[u8]) -> Result<NavPacket, postcard::Error> {
    postcard::from_bytes(bytes)
}
```

Ein typisches Paket (8 SV, Zeitstempel aus ~2023, Korrektur = 0) belegt ≈ 11 Bytes:

- `timestamp`: 9 Bytes (ULEB-128 ~2023)
- `clock_offset`: 1 Byte (zigzag(0) = 0x00)
- `sv_count`: 1 Byte

### JSON ↔ postcard-Kompatibilität

Derselbe Typ unterstützt beide Formate. Die Auswahl erfolgt automatisch über
`is_human_readable()`:

```rust
// JSON (human-readable = true)
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

// postcard (human-readable = false)
let bytes = postcard::to_allocvec(&gps).unwrap();
// [rohe ULEB-128-Bytes, kein Skalen-Tag]

// Beide deserialisieren in denselben Typ:
let from_json: Time<Gps> = serde_json::from_str(&json).unwrap();
let from_postcard: Time<Gps> = postcard::from_bytes(&bytes).unwrap();
assert_eq!(from_json, from_postcard);
```

## defmt-Integration

```rust
use gnss_time::{Time, Gps, DurationParts};

let t = Time::<Gps>::from_week_tow(
    2345,
    DurationParts { seconds: 432_000, nanos: 0 },
).unwrap();
defmt::info!("GPS timestamp: {}", t);
// Ausgabe: GPS 2345:432000.000
```

Alle öffentlichen Typen implementieren `defmt::Format`, wenn das Feature
aktiviert ist (per Kompilierung in CI verifiziert: `embedded.yml` baut
`--features defmt` für jedes Embedded-Ziel):

- `Time<S>` — dasselbe Format wie `Display`
- `Duration` — Format `"Xs Yns"` (dasselbe wie `Display`)
- `GnssTimeError` — kurzer Fehlerstring

## Cross-Kompilierung

Unterstützte Embedded-Ziele (in CI verifiziert, siehe `.github/workflows/embedded.yml`):

| Ziel                            | Architektur                 | Beispiel-Chips             | CI  |
| ------------------------------- | --------------------------- | -------------------------- | --- |
| `thumbv7em-none-eabihf`         | Cortex-M4F/M7F + FPU        | STM32F4/F7, nRF52840       | ✅  |
| `thumbv7em-none-eabi`           | Cortex-M4/M7 ohne FPU       | STM32F3xx                  | ✅  |
| `thumbv6m-none-eabi`            | Cortex-M0/M0+               | STM32F0xx, nRF51           | ✅  |
| `riscv32imac-unknown-none-elf`  | RV32IMAC                    | ESP32-C3, GD32VF103, CH32V | ✅  |
| `riscv32i-unknown-none-elf`     | RV32I (ohne Atomics)        | ESP32-C2                   | ✅  |

Für jedes Ziel verifiziert CI den Build ohne Features und mit dem
`defmt`-Feature. Ein separater CI-Job bestätigt, dass `std` nicht transitiv in
den Abhängigkeitsgraphen gelangt.

Lokaler Check:

```sh
# ARM Cortex-M
cargo check --lib --target thumbv7em-none-eabihf        # STM32F4/F7, nRF52
cargo check --lib --target thumbv7em-none-eabi          # Cortex-M4/M7 ohne FPU
cargo check --lib --target thumbv6m-none-eabi           # Cortex-M0/M0+

# RISC-V
cargo check --lib --target riscv32imac-unknown-none-elf # ESP32-C3
cargo check --lib --target riscv32i-unknown-none-elf    # ESP32-C2

# Mit serde:
cargo check --lib --features serde --target thumbv7em-none-eabihf
```

Die Ziele werden automatisch aus `rust-toolchain.toml` installiert; oder
manuell:

```sh
rustup target add thumbv7em-none-eabihf
rustup target add thumbv6m-none-eabi
rustup target add riscv32imac-unknown-none-elf
rustup target add riscv32i-unknown-none-elf
```

Über `just`:

```sh
just check-no-std           # thumbv7em-none-eabihf
just check-no-std-cortex-m0 # thumbv6m-none-eabi
just check-riscv            # riscv32imac + riscv32i
```

## Muster für Memory-Mapped-Register

```rust
use gnss_time::{Time, Duration, Gps};

// Speichern eines GPS-Zeitstempels in einem 64-Bit-Register oder einer FRAM-Zelle:
fn write_timestamp(reg: &mut u64, t: Time<Gps>) {
    *reg = t.as_nanos();
}

fn read_timestamp(reg: u64) -> Time<Gps> {
    Time::<Gps>::from_nanos(reg)
}
```

## Parsen eines UBX-NAV-TIMEGPS-Pakets

```rust
use gnss_time::{GnssTimeError, Time, Gps, DurationParts};

/// Parst die GPS-Zeit aus einer UBX-NAV-TIMEGPS-Nutzlast (28 Bytes).
pub fn parse_ubx_nav_timegps(payload: &[u8; 28]) -> Result<Time<Gps>, GnssTimeError> {
    // payload ist ein Array mit fester Länge, die Indizes 0..4 / 8..10 sind statisch gültig
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
