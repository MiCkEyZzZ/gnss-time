# Architektur

Interner Aufbau von `gnss-time`.

## Modulaufbau

```text
src/
├── tables/
│   ├── leap_seconds.rs  — BUILTIN_TABLE (19 GPS-Ära-Einträge)
│   └── mod.rs
├── convert.rs      — IntoScale- / IntoScaleWith-Traits + alle Implementierungen
├── civil.rs        — CivilDateTime (ISO 8601 / RFC 3339, aus Time<Utc>)
├── duration.rs     — Duration (vorzeichenbehaftetes Intervall in Nanosekunden)
├── epoch.rs        — CivilDate, konstante Epochen-Offsets, Unix-Offsets
├── error.rs        — GnssTimeError
├── leap.rs         — LeapSecondsProvider, LeapSeconds, alle Konvertierungsfunktionen
├── lib.rs          — Crate-Root, #![no_std], pub use-Reexports
├── matrix.rs       — ConversionMatrix, ScaleId, ConversionKind
├── prelude.rs      — bequeme Reexports
├── scale.rs        — versiegeltes Trait TimeScale + 6 Markertypen
├── serde_impls.rs  — Serialize/Deserialize für Time<S>, Duration, DurationParts
│                     (nur wenn feature = "serde")
└── time.rs         — Time<S>-Struktur, Konstruktoren, Arithmetik, Unix-Methoden
```

## Kerninvariante: TAI als Dreh- und Angelpunkt für Konvertierungen mit festem Offset

Jede Konvertierung mit festem Offset zu TAI läuft über TAI:

```text
T_tai = T_self + S::OFFSET_TO_TAI
T_target = T_tai - Target::OFFSET_TO_TAI
```

Das bedeutet, dass alle paarweisen Konvertierungen zwischen Skalen mit festem
TAI-Offset aus einem einzigen konsistenten Satz von Offsets relativ zu TAI
abgeleitet werden. Dadurch können keine Off-by-one-Fehler zwischen einzelnen
Skalenpaaren entstehen.

Die beiden kontextabhängigen Skalen — UTC und GLONASS — verwenden
`OffsetToTai::Contextual` und besitzen daher keine konstante TAI-Relation.
Dennoch ist GLONASS ↔ UTC selbst eine feste Konvertierung (`IntoScale`):
GLONASS ist über UTC(SU) = UTC + 3 h definiert und wird daher über eine
konstante Epochen-Verschiebung umgerechnet.

Die Offsets (in Nanosekunden) sind Compile-Time-Konstanten, eingebettet in das
Enum `OffsetToTai`:

| Skala   | OFFSET_TO_TAI      |
| ------- | ------------------ |
| GPS     | +19_000_000_000 ns |
| Galileo | +19_000_000_000 ns |
| BeiDou  | +33_000_000_000 ns |
| TAI     | 0                  |
| UTC     | Kontextabhängig    |
| GLONASS | Kontextabhängig    |

## Versiegeltes-Trait-Muster

`TimeScale` ist ein **sealed Trait** — es kann außerhalb dieses Crates nicht
implementiert werden:

```rust
mod private { pub trait Sealed {} }

pub trait TimeScale: private::Sealed + ... { ... }
```

Dies verhindert, dass ein Benutzer eine neue «Pseudo-Zeitskala» erzeugt, die
unbemerkt alle Konvertierungen bricht. Die Menge der unterstützten Skalen ist
fest.

## Speicherdarstellung

`Time<S>` ist exakt 8 Bytes groß (identisch zu `u64`):

```rust
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,  // ZST, belegt keinen Speicher
}
```

- Die Markertypen `S` (`Gps`, `Glonass`, …) sind ebenfalls Zero-Sized Types
- Keine Heap-Allokationen
- Die gesamte Typisierung findet ausschließlich zur Compile-Zeit statt

## Architektur der Schaltsekunden

### Warum expliziter Kontext?

```rust
// ❌ Versteckter Zustand — woher kommen die Schaltsekunden?
let utc = gps.to_utc();

// ✅ Expliziter Kontext — testbar, no_std-kompatibel, deterministisch
let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
```

### Zwei-Pass-Algorithmus UTC → GPS

Eine naive Konvertierung von UTC nach GPS kann in der Nähe einer Schaltsekunde
einen Fehler von ±1 Sekunde verursachen. Die Bibliothek verwendet einen
Zwei-Pass-Algorithmus:

**Pass 1:** TAI wird näherungsweise berechnet, unter der Annahme GPS − UTC = 0

**Pass 2:** Verfeinerung unter Verwendung der Anzahl der Schaltsekunden aus dem
ersten Pass

Dadurch wird der Fehler an den Grenzen aller historischen
Schaltsekundeneinfügungen beseitigt. Die eingebaute Schaltsekundentabelle
enthält 19 Einträge: den Anfangszustand der GPS-Ära (TAI − UTC = 19 s) sowie
18 nachfolgende Schaltsekunden-Übergänge bis TAI − UTC = 37 s (2017-01-01).
Die Tabelle und ihre Tests decken alle 18 Übergänge der GPS-Ära ab.

## Unix-Time-Interoperabilität

`Time<Utc>` zählt Nanosekunden ab **1972-01-01** (UTC-Epoche), während die
Unix-Zeit ab **1970-01-01** zählt. Die Differenz beträgt
`UTC_EPOCH_UNIX_OFFSET_S = 63_072_000 s` (730 Tage):

```text
unix_seconds    = utc_seconds_from_1972 + UTC_EPOCH_UNIX_OFFSET_S
utc_from_1972   = unix_seconds          - UTC_EPOCH_UNIX_OFFSET_S
```

Das ist eine reine Zähl-zu-Zähl-Zuordnung: `Time<Utc>` speichert eine lineare
Nanosekundenzahl ohne Schaltsekunden-Diskontinuitäten. Schaltsekunden werden
nur bei der Konvertierung zwischen Zeitskalen angewendet (siehe oben) — nicht
in dieser Unix-Zuordnung. Diese arbeitet daher auf der internen
Repräsentation und nicht auf einem Schaltsekunden-bewussten zivilen Kalender.

Bereitgestellte Methoden:

| Typ          | Methode                                      |
| ------------ | -------------------------------------------- |
| `Time<Utc>`  | `from_unix_seconds(i64) -> Result<Self>`     |
| `Time<Utc>`  | `from_unix_nanos(i64)   -> Result<Self>`     |
| `Time<Utc>`  | `as_unix_seconds() -> i64`                   |
| `Time<Utc>`  | `as_unix_nanos()   -> i64`                   |
| `Time<Gps>`  | `from_unix_seconds(i64, P) -> Result<Self>`  |
| `Time<Gps>`  | `as_unix_seconds(P) -> Result<i64>`          |

## Serde-Unterstützung (feature = "serde")

Aktivierung:

```toml
gnss-time = { version = "0.8", features = ["serde"] }
```

### Formate

#### `Time<S>`

**Human-readable** (JSON, TOML, YAML):

```json
{ "scale": "GPS", "nanos": 1356566418000000000 }
```

Das Feld `scale` wird bei der Deserialisierung validiert — der Versuch,
`{ "scale": "UTC", ... }` in `Time<Gps>` zu deserialisieren, führt zu einem
Fehler.

**Compact** (postcard, bincode, MessagePack): ein roher `u64`-Wert in
Nanosekunden ohne Skalen-Tag. Die Skala wird vom Typsystem getragen.

#### `Duration`

| Format         | Form                         |
| -------------- | ---------------------------- |
| Human-readable | `{ "nanos": -7000000000 }`   |
| Compact        | roher `i64`-Wert             |

#### `DurationParts`

| Format         | Form                                 |
| -------------- | ------------------------------------ |
| Human-readable | `{ "seconds": 5, "nanos": 500000000 }` |
| Compact        | 2-Element-Tupel `[u64, u32]`         |

`DurationParts` ist ein eigener, vorzeichenloser Teile-Typ
(`seconds: u64`, `nanos: u32`) für GNSS-Wochen-/Tages-Konstruktoren. Er
kodiert niemals ein Vorzeichen — negative Intervalle gibt es nur in
`Duration` selbst (kompakt als i64).

### Implementierungsprinzipien

- **Kein Proc-Macro** — die Implementierungen verwenden direkt die serde
  Visitor-API
- **no_std-kompatibel** — `serde` wird mit `default-features = false`
  eingebunden
- `is_human_readable()` bestimmt das Format zur Laufzeit — eine
  Implementierung funktioniert sowohl mit JSON als auch mit postcard
- Fehler bei der Skalenvalidierung benötigen kein `alloc` — `fmt::Display`
  wird verwendet

```rust
// Beispiel — JSON-Roundtrip
let gps = Time::<Gps>::from_seconds(1_356_566_418);
let json = serde_json::to_string(&gps).unwrap();
// {"scale":"GPS","nanos":1356566418000000000}

let back: Time<Gps> = serde_json::from_str(&json).unwrap();
assert_eq!(gps, back);

// Beispiel — postcard-Roundtrip
let bytes = postcard::to_allocvec(&gps).unwrap();
let back: Time<Gps> = postcard::from_bytes(&bytes).unwrap();
assert_eq!(gps, back);
```

Hinweis: Der `gnss-time`-Serde-Code selbst bleibt `no_std`-kompatibel; der
Aufruf `postcard::to_allocvec()` im Beispiel benötigt zusätzlich `alloc`
(das `alloc`-Feature der `postcard`-Crate).

## Feature-Flags

| Feature | Wirkung                                            |
| ------- | -------------------------------------------------- |
| (none)  | Reines `no_std`, keine externen Abhängigkeiten     |
| `std`   | `impl std::error::Error for GnssTimeError`         |
| `serde` | `Serialize`/`Deserialize` für alle öffentlichen Typen |
| `alloc` | Reserviert (No-op) — Heap-basierte Serde-Fehlermeldungen sind geplant |
| `defmt` | `impl defmt::Format` für alle öffentlichen Typen   |

## Design der Konvertierungs-Traits

```rust
// Fester Offset — GPS ↔ TAI, GPS ↔ Galileo, GLONASS ↔ UTC
pub trait IntoScale<Target: TimeScale>: Sized {
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}

// Kontextabhängige Konvertierungen — GPS ↔ UTC, GPS ↔ GLONASS usw.
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    fn into_scale_with<P: LeapSecondsProvider>(self, ls: P)
        -> Result<Time<Target>, GnssTimeError>;
    fn into_scale_with_checked<P: LeapSecondsProvider>(self, ls: P)
        -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}
```

`ConvertResult<T>` fügt ein Signal darüber hinzu, ob das
Schaltsekunden-Mehrdeutigkeitsfenster getroffen wurde.

## CI-Garantien

| Prüfung                        | Werkzeug                                                       |
| ------------------------------ | -------------------------------------------------------------- |
| Kein unsicherer Code           | `#![forbid(unsafe_code)]`                                      |
| Kein undokumentiertes API      | `#![deny(missing_docs)]`                                       |
| Build für Embedded-Ziele       | `cargo check --target thumbv7em-none-eabihf`                   |
| Typgröße = 8 Bytes             | Unit-Test `test_size_equals_u64`                               |
| Sichere Arithmetik             | `-D warnings` + Abwesenheit von `#[allow(arithmetic_overflow)]` |
| Serde-Roundtrip (JSON)         | Tests in `src/serde_impls.rs`                                  |
| Serde-Roundtrip (postcard)     | Tests in `src/serde_impls.rs`                                  |
