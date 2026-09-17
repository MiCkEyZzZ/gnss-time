# Architektur

Interner Aufbau von `gnss-time`.

## Inhaltsverzeichnis

- [Schichtenarchitektur](#schichtenarchitektur)
- [Modulaufbau](#modulaufbau)
- [Modulabhängigkeitsdiagramm](#modulabhängigkeitsdiagramm)
- [Kerninvariante: TAI als Pivot für Konvertierungen mit festem Offset](#kerninvariante-tai-als-pivot-für-konvertierungen-mit-festem-offset)
- [Zwei Klassen von Konvertierungen: Fest vs. Kontextabhängig](#zwei-klassen-von-konvertierungen-fest-vs-kontextabhängig)
- [Das Sealed-Trait-Muster](#das-sealed-trait-muster)
- [Speicherdarstellung](#speicherdarstellung)
- [Architektur der Schaltsekunden](#architektur-der-schaltsekunden)
- [Unix-Time-Interoperabilität](#unix-time-interoperabilität)
- [Datum und Uhrzeit (ISO 8601)](#datum-und-uhrzeit-iso-8601)
- [Serde-Unterstützung](#serde-unterstützung-feature--serde)
- [Feature-Flags](#feature-flags)
- [Erweiterung: Hinzufügen einer neuen Zeitskala](#erweiterung-hinzufügen-einer-neuen-zeitskala)
- [Grenzen](#grenzen)
- [CI-Garantien](#ci-garantien)

---

## Schichtenarchitektur

`gnss-time` ist in fünf Schichten organisiert. Die Abhängigkeiten zwischen den
Schichten sind azyklisch: jede Schicht hängt nur von den Schichten unter ihr ab.

```text
┌─────────────────────────────────────────────────────────────────┐
│  5. matrix         ConversionMatrix, ScaleId — Laufzeit-        │
│                    Introspektion des Konvertierungsgraphen      │
├─────────────────────────────────────────────────────────────────┤
│  4. convert        IntoScale, IntoScaleWith, ConvertResult —    │
│                    die öffentliche Konvertierungs-API           │
├─────────────────────────────────────────────────────────────────┤
│  3. leap           LeapSecondsProvider, LeapSeconds,            │
│                    RuntimeLeapSeconds — die kontextabhängigen   │
│                    (schaltsekundenbewussten) Konvertierungs-    │
│                    funktionen                                   │
├─────────────────────────────────────────────────────────────────┤
│  2. time           Time<S> — der zentrale Werttyp, Arithmetik,  │
│                    Konvertierungen mit festem Offset            │
│                    (to_tai/from_tai)                            │
├─────────────────────────────────────────────────────────────────┤
│  1. scale + epoch  TimeScale-Trait, Markertypen (Gps, Utc,      │
│                    …), CivilDate, Epochen-Offset-Konstanten     │
└─────────────────────────────────────────────────────────────────┘
```

**Schicht 1 (`scale`, `epoch`)** definiert, *was eine Zeitskala ist*: ein
Zero-Sized-Markertyp plus eine Compile-Zeit-Beziehung zu TAI. `epoch` liefert
die Kalenderarithmetik (`CivilDate`), mit der diese Beziehungen hergeleitet
werden, und ist im Übrigen unabhängig von `scale`.

**Schicht 2 (`time`)** definiert `Time<S>`, den einzigen Werttyp der Crate,
und alles, was keine externen Daten erfordert: Konstruktion, Arithmetik
(`+`, `-`, `checked_add`, …) und Konvertierungen zwischen Skalen, die einen
*festen* Offset zu TAI teilen (`to_tai`, `from_tai`, `try_convert`).

**Schicht 3 (`leap`)** fügt die kontextabhängigen Konvertierungen hinzu —
jene, die eine an der Aufrufstelle übergebene Schaltsekundentabelle benötigen
(`gps_to_utc`, `utc_to_gps` und die daraus abgeleiteten GLONASS/Galileo/BeiDou-
Varianten). Diese Schicht führt das `LeapSecondsProvider`-Trait und seine zwei
konkreten Implementierungen ein (plus ein Blanket-`&P`-impl).

**Schicht 4 (`convert`)** ist die öffentliche, ergonomische Eingangstür:
`IntoScale`/`IntoScaleWith` kapseln die Funktionen der Schichten 2/3 hinter
einer einheitlichen, traitorientierten API und fügen `ConvertResult` für die
Berichterstattung über Schaltsekunden-Mehrdeutigkeiten hinzu.

**Schicht 5 (`matrix`)** ist Dokumentation-als-Code: Ihre Hauptaufgabe ist,
jedes Skalenpaar (`Fixed`/`Identity`/`EpochShift`/`Contextual`/`SameScale`)
für die Laufzeit-Introspektion und für die erschöpfenden paarweisen Tests zu
klassifizieren, die dieses Dokument ehrlich halten. Sie führt selbst keine
Konvertierungen durch, mit einer einzigen Ausnahme:
`beidou_via_gps_to_glonass_via_utc` ist eine echte Konvertierungskette, die
hier gehalten wird, damit `ConversionChain` die GPS/UTC/TAI-Zwischenwerte
tragen kann, die das Introspektionsbeispiel benötigt.

Zwei Module liegen außerhalb dieses Stapels, weil sie optional bzw. rein
additiv sind:

- **`civil`** fügt eine menschenlesbare Kalenderansicht (`CivilDateTime`)
  über `Time<Utc>` hinzu; es nimmt an keiner Konvertierung teil.
- **`serde_impls`** (feature-gated) fügt `Serialize`/`Deserialize` hinzu; es
  besitzt keine eigene Konvertierungslogik.

## Modulaufbau

```text
src/
├── tables/
│   ├── leap_seconds.rs  — BUILTIN_TABLE (19 GPS-Ära-Einträge)
│   └── mod.rs
├── scale.rs         — Schicht 1: versiegeltes Trait TimeScale + 6 Markertypen
├── epoch.rs         — Schicht 1: CivilDate, konstante Epochen-Offsets, Unix-Offsets
├── time.rs          — Schicht 2: Time<S>-Struktur, Konstruktoren, Arithmetik,
│                      Konvertierung mit festem Offset (to_tai/from_tai), Unix-Methoden
├── leap.rs          — Schicht 3: LeapSecondsProvider, LeapSeconds,
│                      RuntimeLeapSeconds, alle kontextabhängigen
│                      Konvertierungsfunktionen
├── convert.rs       — Schicht 4: IntoScale-/IntoScaleWith-Traits + alle
│                      Implementierungen pro Skalenpaar, ConvertResult
├── matrix.rs        — Schicht 5: ConversionMatrix, ScaleId, ConversionKind
├── civil.rs         — CivilDateTime (ISO 8601 / RFC 3339, aus Time<Utc>)
├── error.rs         — GnssTimeError (von jeder Schicht verwendet)
├── serde_impls.rs   — Serialize/Deserialize für Time<S>, Duration,
│                      DurationParts (nur wenn feature = "serde")
├── duration.rs      — Duration (vorzeichenbehaftetes Intervall in Nanosekunden;
│                      keine Abhängigkeit zu scale/time — reiner Arithmetiktyp)
├── prelude.rs       — bequeme Reexports
└── lib.rs           — Crate-Root, #![no_std], pub use-Reexports
```

`duration.rs` ist nicht Teil des Fünf-Schichten-Stapels oben: `Duration`
stellt ein *Intervall* dar, keinen Zeitpunkt, und hat keinerlei Beziehung zu
`TimeScale`. Es wird von Schicht 2 (Arithmetik von `Time<S>`) verwendet,
hängt aber nicht von ihr ab.

## Modulabhängigkeitsdiagramm

```text
                        ┌───────────┐
                        │  duration │  (keine crate-internen Abhängigkeiten
                        └─────┬─────┘   außer error)
                              │
 ┌───────────┐   ┌───────────┐│┌───────────┐
 │   epoch   │──▶│   scale   │┴│   error   │  (Blätter — keine crate-internen
 └───────────┘   └─────┬─────┘ └─────┬─────┘   Abhängigkeiten außer der
                       │             │          Abwesenheit voneinander)
                       ▼             │
                 ┌───────────┐       │
                 │    time   │◀──────┘
                 └─────┬─────┘
                       │
                       ▼
            ┌───────────┐   ┌───────────┐
            │   tables  │──▶│    leap   │
            └───────────┘   └─────┬─────┘
                                  │
                                  ▼
                            ┌───────────┐
                            │  convert  │
                            └─────┬─────┘
                                  │
                                  ▼
                            ┌───────────┐
                            │   matrix  │
                            └───────────┘

            ┌───────────┐                  ┌─────────────────┐
  time ────▶│   civil   │◀──── time        │  serde_impls    │────▶ scale
            └───────────┘                  │ (feature=serde) │────▶ time
                                           └─────────────────┘
            (civil nimmt an keiner Konvertierung teil)
```

Pfeile bedeuten „hängt ab von". `error` wird von jeder Schicht verwendet (alle
fehlbaren Operationen geben `GnssTimeError` zurück) und ist in den Pfeilen
oben aus Gründen der Lesbarkeit weggelassen — außer dort, wo es selbst ein
Blatt ist.

Beachte den Zyklus `time` ↔ `civil`: `Time<Utc>` besitzt
eine `to_civil()`-Komfortmethode, die `CivilDateTime` zurückgibt, und `civil`
baut `CivilDateTime` aus `Time<Utc>`. Beide Module verweisen aufeinander. Das
ist ein Zyklus auf Modulebene innerhalb der Crate (in Rust zulässig), der
ausschließlich für die Kalenderansicht-Komfortfunktion existiert; es ist keine
Konvertierungsabhängigkeit.

## Kerninvariante: TAI als Pivot für Konvertierungen mit festem Offset

Jede Konvertierung zwischen zwei Skalen, die beide einen **festen** Offset zu
TAI besitzen, läuft über TAI als Zwischenwert:

```text
T_tai    = T_self   + S::OFFSET_TO_TAI
T_target = T_tai     - Target::OFFSET_TO_TAI
```

Konkret ist das `Time::<S>::to_tai`, gefolgt von `Time::<Target>::from_tai`,
zusammengesetzt als `Time::<S>::try_convert::<Target>()`. Das bedeutet, dass
**alle** paarweisen Konvertierungen zwischen Skalen mit festem Offset aus
einem einzigen konsistenten Satz von Offsets relativ zu TAI abgeleitet werden —
es kann keine Off-by-one-Fehler zwischen einzelnen Skalenpaaren geben, weil
kein Paar speziell behandelt wird. GPS ↔ Galileo, GPS ↔ BeiDou, GPS ↔ TAI,
Galileo ↔ BeiDou und Galileo/BeiDou ↔ TAI sind alle *dieselbe*
Zwei-Zeilen-Komposition mit unterschiedlichen Konstanten.

Die beiden kontextabhängigen Skalen — UTC und GLONASS — verwenden
`OffsetToTai::Contextual`, besitzen also keine konstante TAI-Beziehung und
können nicht an `try_convert` teilnehmen. GLONASS ↔ UTC ist dennoch weiterhin
eine **feste** Konvertierung auf der `IntoScale`-Ebene (siehe unten): GLONASS
ist über UTC(SU) = UTC + 3 h definiert, eine konstante Epochenverschiebung,
die überhaupt nicht über TAI läuft.

Die Offsets (in Nanosekunden) sind Compile-Time-Konstanten, eingebettet in das
Enum `OffsetToTai`:

| Skala   | `OFFSET_TO_TAI`             |
| ------- | --------------------------- |
| GPS     | `Fixed(+19 000 000 000)` ns |
| Galileo | `Fixed(+19 000 000 000)` ns |
| BeiDou  | `Fixed(+33 000 000 000)` ns |
| TAI     | `Fixed(0)`                  |
| UTC     | `Contextual`                |
| GLONASS | `Contextual`                |

Dass GPS und Galileo denselben festen Offset teilen, macht ihre Konvertierung
zur **Identität** auf der zugrunde liegenden Nanosekundenzahl (siehe
`ConversionKind::Identity` in `matrix.rs`) — es ist überhaupt keine Arithmetik
nötig, nur ein Wechsel des Phantom-Typ-Parameters.

## Zwei Klassen von Konvertierungen: Fest vs. Kontextabhängig

Jedes geordnete Skalenpaar fällt in genau eine von fünf Arten, klassifiziert
durch `ScaleId::conversion_kind` in `matrix.rs`:

| Art         | Erfordert externe Daten?           | Trait          | Beispiel                      |
| ----------- | ---------------------------------- | -------------- | ----------------------------- |
| `SameScale` | Nein                               | n/a            | `Gps → Gps`                   |
| `Identity`  | Nein                               | `IntoScale`    | `Gps ↔ Galileo`               |
| `Fixed`     | Nein                               | `IntoScale`    | `Gps ↔ Tai`, `Gps ↔ Beidou`   |
| `EpochShift`| Nein                               | `IntoScale`    | `Glonass ↔ Utc`               |
| `Contextual`| **Ja** — ein `LeapSecondsProvider` | `IntoScaleWith`| `Gps ↔ Utc`, `Gps ↔ Glonass`  |

**Feste Konvertierungen** (`Identity`, `Fixed`, `EpochShift` — gemeinsam
`ScaleId::is_fixed`) sind reine Funktionen ihrer Eingabe: keine
Schaltsekundentabelle, keine fehlbare externe Suche, nur Überlauf kann sie zum
Scheitern bringen. Sie implementieren [`IntoScale`], dessen Signatur keine
solche Abhängigkeit trägt:

```rust
pub trait IntoScale<Target: TimeScale>: Sized {
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}
```

**Kontextabhängige Konvertierungen** erfordern die Kenntnis des aktuellen
TAI − UTC-Offsets, der keine Compile-Time-Konstante ist — er ändert sich bei
jeder von der IERS angesetzten Schaltsekunde. Diese Abhängigkeit hinter
globalem veränderlichem Zustand zu verstecken, würde die `no_std`-
Unterstützung brechen, Tests von Mocks abhängig machen und das Ergebnis davon
abhängen lassen, *wann* die Crate kompiliert wurde, statt von expliziter,
prüfbarer Eingabe. Stattdessen nimmt jede kontextabhängige Konvertierung einen
`LeapSecondsProvider` explizit entgegen:

```rust
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    fn into_scale_with<P: LeapSecondsProvider>(self, ls: P)
        -> Result<Time<Target>, GnssTimeError>;
    fn into_scale_with_checked<P: LeapSecondsProvider>(self, ls: P)
        -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}
```

```rust
// ❌ Versteckter Zustand — woher kommen die Schaltsekunden?
let utc = gps.to_utc();

// ✅ Expliziter Kontext — testbar, no_std-kompatibel, deterministisch
let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
```

`into_scale_with_checked` meldet zusätzlich, ob das Ergebnis innerhalb des
Ein-Sekunden-Fensters um eine Schaltsekunden-Einfügung liegt, über
`ConvertResult<T>`:

```rust
pub enum ConvertResult<T> {
    Exact(T),
    AmbiguousLeapSecond(T),
}
```

Außerhalb dieses Fensters ist jede kontextabhängige Konvertierung exakt
zyklisch (`A → B → A == A`); innerhalb ist die Abbildung von GPS/Galileo/BeiDou-
Zeit auf UTC nicht injektiv (zwei aufeinanderfolgende GPS-Sekunden können
dieselbe zivile UTC-Sekunde ergeben), weshalb `Exact` nicht garantiert werden
kann und `into_scale_with_checked` genau dafür existiert: Aufrufer können diesen
Fall erkennen und behandeln, statt stillschweigend einem Näherungsergebnis zu
vertrauen.

### Der Zwei-Pass-Algorithmus UTC → GPS

Eine naive UTC → GPS-Konvertierung erzeugt in der Nähe einer
Schaltsekunden-Einfügung einen Fehler von ±1 Sekunde, weil die anzuwendende
Korrektur davon abhängt, auf welcher Seite der Einfügung das *Ergebnis* liegt —
was man genau noch nicht weiß. Die Bibliothek löst dies mit einem
Zwei-Pass-Algorithmus in `utc_to_gps`:

**Pass 1:** TAI wird näherungsweise berechnet, unter der Annahme GPS − UTC = 0.

**Pass 2:** Die aus Pass 1 abgeleitete Schaltsekundenzahl wird verwendet, um
das Ergebnis zu verfeinern.

Dadurch wird der Fehler an den Grenzen aller 18 historischen
Schaltsekunden-Einfügungen beseitigt, die von der eingebauten Tabelle abgedeckt
werden; die Testsuite und die Fuzz-Ziele `fuzz_gps_utc`/`fuzz_utc_to_gps`
üben jede einzelne von ihnen aus.

## Das Sealed-Trait-Muster

`TimeScale` ist ein versiegeltes Trait — es kann außerhalb dieser Crate nicht
implementiert werden:

```rust
mod private { pub trait Sealed {} }

pub trait TimeScale: private::Sealed + Copy + Clone + Eq + PartialEq + Debug {
    const NAME: &'static str;
    const OFFSET_TO_TAI: OffsetToTai;
    const EPOCH_CIVIL: CivilDate;
    const DISPLAY_STYLE: DisplayStyle;
}
```

`private::Sealed` ist nur für die eigenen sechs Markertypen der Crate
implementiert (`Gps`, `Glonass`, `Galileo`, `Beidou`, `Tai`, `Utc`), und da
`Sealed` in einem privaten Modul deklariert ist, kann keine
Downstream-Crate es benennen — und daher kein `impl TimeScale for MyScale`
schreiben.

Das ist ein bewusstes Closed-World-Design: Jeder `TimeScale`-Implementierer
muss sein `OFFSET_TO_TAI` korrekt angeben und in den Konvertierungsgraphen in
`convert.rs` und `matrix.rs` passen. Könnten externe Typen das Trait
implementieren, müsste `try_convert::<T>()` Skalen verarbeiten, deren
Offsetbeziehung die Crate nicht verifizieren kann, und die obige Invariante
„alle paarweisen Konvertierungen teilen einen konsistenten TAI-Pivot“
wäre von der eigenen Testsuite der Crate nicht mehr überprüfbar. Siehe
[Erweiterung](#erweiterung-hinzufügen-einer-neuen-zeitskala) dafür, was das
Hinzufügen einer *neuen, von der Crate gepflegten* Skala tatsächlich erfordert.

## Speicherdarstellung

`Time<S>` ist exakt 8 Bytes groß, identisch zu einem nackten `u64`:

```rust
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,   // ZST, belegt keinen Speicher
}
```

- Die Markertypen `S` (`Gps`, `Glonass`, …) sind Zero-Sized.
- Keine Heap-Allokationen im eigenen Code der Crate (ohne das `serde`-Feature;
  auch `serde_impls` selbst allokiert nicht — siehe unten).
- Die gesamte Typ-Level-Skalenprüfung findet nur zur Compile-Zeit statt; zur
  Laufzeit wird über die acht Bytes von `nanos` hinaus nichts gespeichert.

Dies wird durch `test_size_equals_u64` im Testmodul von `time.rs` und durch die
Size-Probe-Crate `firmware/` (`docs/EMBEDDED.md`) verifiziert, die die
kompilierte `.text`-Größe repräsentativer Operationen auf
`thumbv7em-none-eabihf` misst.

## Architektur der Schaltsekunden

Die eingebaute Schaltsekundentabelle enthält 19 Einträge: den Anfangszustand
der GPS-Ära (TAI − UTC = 19 s zur GPS-Epoche, 1980-01-06) plus 18
nachfolgende Schaltsekunden-Einfügungen, endend bei TAI − UTC = 37 s
(2017-01-01, die jüngste zum Zeitpunkt dieses Dokuments).
`tables/leap_seconds.rs` bettet Compile-Zeit-Assertions (`const`-Blöcke) ein,
die prüfen, dass die Tabelle strikt nach Schwelle sortiert ist und jeder
Eintrag den Offset um exakt 1 erhöht — eine fehlerhafte eingebaute Tabelle
scheitert bei `cargo build`, nicht erst bei `cargo test`.

Es existieren drei Implementierungen von `LeapSecondsProvider`:

- **`LeapSeconds`** — kapselt die statische `&'static [LeapEntry]`-Tabelle
  (oder ein beliebiges anderes `'static`-Slice über
  `from_table`/`try_from_slice`). Kostenfrei, keine Laufzeit-Mutation.
- **`RuntimeLeapSeconds`** — ein Puffer mit fester Kapazität
  (`RUNTIME_CAPACITY = 64`) und ohne Heap, der zur Laufzeit über `try_extend`
  erweitert werden kann, für Empfänger, die neue
  Schaltsekunden-Ankündigungen aus einer Navigationsnachricht lernen.
  `try_extend` und `try_from_slice`/`from_slice` erzwingen alle dieselben
  Ordnungs- und Einerschritt-Invarianten wie die Compile-Zeit-Tabelle — siehe
  `fuzz_try_extend.rs` für das Fuzz-Harness, das diesen Vertrag festnagelt,
  einschließlich des während dieses Audits gefundenen und behobenen
  `OffsetOverflow`-Falls (siehe [Fuzzing](../fuzz/README.md)).
- **`&P`** — ein Blanket-`impl<P: LeapSecondsProvider> LeapSecondsProvider
  for &P`, sodass eine Referenz auf eine der obigen Implementierungen überall
  dort übergeben werden kann, wo ein Provider-Argument erwartet wird
  (z. B. `gps_to_utc(t, &ls)`).

## Unix-Time-Interoperabilität

`Time<Utc>` zählt Nanosekunden ab **1972-01-01** (der UTC-Epoche), während die
Unix-Zeit ab **1970-01-01** zählt. Die Differenz beträgt
`UTC_EPOCH_UNIX_OFFSET_S = 63_072_000 s` (730 Tage):

```text
unix_seconds    = utc_seconds_from_1972 + UTC_EPOCH_UNIX_OFFSET_S
utc_from_1972   = unix_seconds          - UTC_EPOCH_UNIX_OFFSET_S
```

Das ist eine reine Zählabbildung — `Time<Utc>` speichert eine lineare
Nanosekundenzahl ohne eigene Schaltsekunden-Diskontinuitäten. Schaltsekunden
werden nur bei der *Konvertierung zwischen Zeitskalen* angewendet (der
vorherige Abschnitt); die Unix-Zuordnung arbeitet rein auf der internen
linearen Darstellung und berührt niemals einen `LeapSecondsProvider`.

| Typ          | Methode                                      |
| ------------ | -------------------------------------------- |
| `Time<Utc>`  | `from_unix_seconds(i64) -> Result<Self>`     |
| `Time<Utc>`  | `from_unix_nanos(i64) -> Result<Self>`       |
| `Time<Utc>`  | `as_unix_seconds() -> i64`                   |
| `Time<Utc>`  | `as_unix_nanos() -> i64`                     |
| `Time<Gps>`  | `from_unix_seconds(i64, P) -> Result<Self>`  |
| `Time<Gps>`  | `as_unix_seconds(P) -> Result<i64>`          |

Die `Time<Gps>`-Varianten erfordern einen `LeapSecondsProvider`, weil sie über
`Time<Utc>` und dann eine kontextabhängige GPS↔UTC-Konvertierung laufen; die
`Time<Utc>`-Varianten tun dies nicht, weil sie die UTC-Skala nie verlassen.

## Datum und Uhrzeit (ISO 8601)

`civil::CivilDateTime` ist eine menschenlesbare Ansicht, die aus `Time<Utc>`
abgeleitet wird: Jahr/Monat/Tag/Stunde/Minute/Sekunde/Nanosekunde-Felder plus
eine `Display`-Implementierung, die RFC 3339 / ISO 8601-Text erzeugt
(`2024-01-15T12:34:56.123456789Z`). Sie hängt nur von `Time<Utc>` ab und nimmt
an keiner Skalenkonvertierung teil — `to_civil()`/`CivilDateTime::to_utc()`
sind ein verlustfreier, allokationsfreier Roundtrip, der niemals eine
Schaltsekundentabelle berührt, weil die eigene Nanosekundenzahl von `Time<Utc>`
keine der Schaltsekunden-Diskontinuitäten enthält, die eine
Wanduhr-Kalenderanzeige sonst berücksichtigen müsste.

Wie im [Abhängigkeitsdiagramm](#modulabhängigkeitsdiagramm) angemerkt, ist die
Beziehung zwischen `time` und `civil` auf Modulebene bidirektional: `Time<Utc>`
erhält die `to_civil()`-Komfortfunktion, und `civil` baut auf `Time<Utc>` auf.

## Serde-Unterstützung (`feature = "serde"`)

```toml
gnss-time = { version = "0.9", features = ["serde"] }
```

### Formate

`Time<S>` — **human-readable** (JSON, TOML, YAML):

```json
{ "scale": "GPS", "nanos": 1356566418000000000 }
```

Das Feld `scale` wird bei der Deserialisierung validiert: Deserialisieren von
`{ "scale": "UTC", … }` in `Time<Gps>` führt zu einem Fehler.

`Time<S>` — **compact** (postcard, bincode, MessagePack): ein roher `u64`-Wert
in Nanosekunden ohne Skalen-Tag — die Skala wird vollständig vom Rust-Typ
getragen, sodass es auf dieser Ebene nichts zu validieren oder zu beschädigen
gibt.

`Duration` und `DurationParts` folgen derselben Human-readable/Compact-
Aufteilung:

| `Duration`     | Form                          |
| -------------- | ----------------------------- |
| Human-readable | `{ "nanos": -7000000000 }`    |
| Compact        | roher `i64`-Wert              |

| `DurationParts`     | Form                                   |
| ------------------- | -------------------------------------- |
| Human-readable      | `{ "seconds": 5, "nanos": 500000000 }` |
| Compact             | 2-Element-Tupel `[u64, u32]`           |

`DurationParts` ist ein eigener, nicht-negativer Teile-Typ (`seconds: u64`,
`nanos: u32`), der von den GNSS-Wochen-/Tages-Konstruktoren verwendet wird —
er kodiert niemals ein Vorzeichen; negative Intervalle existieren nur in
`Duration` selbst. Siehe `docs/EMBEDDED.md` für das exakte Drahtformat und die
Byte-Größen-Tabelle.

### Implementierungsprinzipien

- **Kein Proc-Macro** — die Implementierungen werden von Hand gegen die
  `serde`-Visitor-API geschrieben, sodass die `no_std`-Garantie der Crate nicht
  von den Codegenerierungsannahmen von `serde_derive` abhängt.
- **`no_std`-kompatibel** — `serde` wird mit `default-features = false`
  eingebunden.
- `is_human_readable()` wählt das Format zur Laufzeit, sodass eine
  Implementierung sowohl JSON als auch postcard bedient.
- Fehler bei Skalenabweichungen benötigen kein `alloc` — sie werden über
  `fmt::Display` erzeugt, nicht über `String`-Formatierung.

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

| Feature | Wirkung                                                          |
| ------- | ---------------------------------------------------------------- |
| (none)  | Reines `no_std`, keine externen Abhängigkeiten                   |
| `std`   | `impl std::error::Error for GnssTimeError`                       |
| `serde` | `Serialize`/`Deserialize` für alle öffentlichen Typen            |
| `alloc` | Reservierter No-op — heap-gestützte Serde-Fehlermeldungen geplant|
| `defmt` | `impl defmt::Format` für alle öffentlichen Typen                 |

## Erweiterung: Hinzufügen einer neuen Zeitskala

Das Hinzufügen einer Skala bedeutet, einen neuen, crate-internen
Implementierer des versiegelten `TimeScale`-Traits über das `define_scale!`-
Makro in `scale.rs` hinzuzufügen und ihn dann in den Konvertierungsgraphen
einzubinden. Konkret, für eine Skala, deren Beziehung zu TAI ein **fester**
Offset ist (der häufige Fall — siehe [QZSS/NavIC](../docs/GNSS_TIME_PRIMER.md)
als ausgeführtes Beispiel, Roadmap-Punkt #TIME-32):

1. **`scale.rs`** — füge einen `define_scale!`-Aufruf hinzu, der `NAME`,
   `OFFSET_TO_TAI` (`Fixed(offset_ns)` oder `Contextual`), `EPOCH_CIVIL` und
   `DISPLAY_STYLE` liefert.
2. **`convert.rs`** — implementiere `IntoScale<NewScale>` für jede bestehende
   Skala mit *festem* Offset, zu der/ von der sie direkt konvertieren soll
   (oder verlasse dich auf die Blanket-`try_convert::<T>()`-Komposition über
   TAI, wenn kein direktes, dokumentiertes `impl` erforderlich ist). Ist die
   neue Skala stattdessen kontextabhängig (wie UTC/GLONASS), implementiere
   `IntoScaleWith<NewScale>` und leite über eine bestehende kontextabhängige
   Konvertierung — genauso, wie `Galileo`/`Beidou`→`Utc` über `Gps`→`Utc`
   laufen (siehe den `#TIME-27.1`-Auditfix in `CHANGELOG.md` dafür, warum das
   Leiten über einen einzelnen mehrdeutigkeitsbewussten Pfad wichtig ist,
   statt die Schaltsekundenlogik pro Skala neu abzuleiten).
3. **`matrix.rs`** — füge die neue `ScaleId`-Variante hinzu, erweitere
   `ScaleId::ALL` und erweitere den `conversion_kind`-Match, sodass jedes Paar
   mit der neuen Skala klassifiziert wird. `ConversionMatrix::path_count` und
   die erschöpfenden paarweisen Tests in `matrix.rs` werden sie dann
   automatisch mit einbeziehen.
4. **Tests** — füge Roundtrip-Tests hinzu (`new_scale → existing_scale →
   new_scale` erhält die Nanosekunden für Identitäts-/Feste-Beziehungen) und,
   für eine kontextabhängige Skala, einen Mehrdeutigkeitsfenster-Test, der
   `fuzz_gps_utc`/`fuzz_utc_to_gps` nachbildet.
5. **Doku** — füge eine Zeile zur Offsettabelle in
   [Kerninvariante](#kerninvariante-tai-als-pivot-für-konvertierungen-mit-festem-offset)
   und zu `docs/GNSS_TIME_PRIMER.md` hinzu.

Da `TimeScale` versiegelt ist, ist diese Liste konstruktionsbedingt
erschöpfend: Eine Skala, die Schritt 3 überspringt, erscheint einfach nicht in
`ConversionMatrix`, und die eigenen paarweisen Vollständigkeitstests der Matrix
werden eine *fehlende* Skala nicht erkennen (sie iterieren über
`ScaleId::ALL`, sodass eine nicht gelistete Skala für sie unsichtbar ist) —
genau deshalb muss Schritt 3 eine bewusste, überprüfte Ergänzung sein, statt
etwas, das das Typsystem automatisch erzwingt.

## Grenzen

**Warum gibt es kein `From<Time<Gps>> for Time<Utc>`?**

`From`/`Into` sind in Rust als *infallibel* dokumentiert und werden
konventionsgemäß als billige Konvertierungen erwartet, bei denen „kein Weg zum
Scheitern“ besteht. GPS→UTC ist keins von beidem: Es erfordert einen expliziten
`LeapSecondsProvider` (es gibt keinen Default, den das Trait erreichen könnte,
ohne den versteckten globalen Zustand wieder einzuführen, den diese Crate
bewusst vermeidet — siehe
[Fest vs. Kontextabhängig](#zwei-klassen-von-konvertierungen-fest-vs-kontextabhängig)
oben), und selbst mit einem Provider kann das Ergebnis für Werte außerhalb des
darstellbaren Bereichs von `Time` `Overflow` sein oder innerhalb eines
Schaltsekundenfensters mehrdeutig. Nichts davon passt zum `From`-Vertrag, also
stellt die Crate stattdessen `IntoScaleWith`/`gps_to_utc` bereit, die sowohl
den erforderlichen Kontext als auch die Möglichkeit des Scheiterns explizit in
der Signatur machen.

Auch die Konvertierungen mit **festem** Offset (`Gps → Tai`, `Gps → Galileo`, …)
verwenden `IntoScale` statt `TryFrom`, aus einem engeren Grund: `TryFrom`
würde für *ein* festes Paar funktionieren, aber die Crate möchte genau ein
Trait, dessen `impl`s über `ScaleId`/`ConversionMatrix` für die
Introspektionsgeschichte in Schicht 5 aufzählbar sind — das Mischen von
`TryFrom` (fest) und einem benutzerdefinierten Trait (kontextabhängig) würde
diese Geschichte in „prüfe `TryFrom` für dieses Paar, prüfe `IntoScaleWith` für
jenes“ zerfasern, ohne eine einzige Quelle der Wahrheit dafür, „welche
Konvertierungen existieren.“ `IntoScale`/`IntoScaleWith` geben beiden Klassen
eine einheitliche Form, die `matrix.rs` erschöpfend beschreiben kann.

**Andere Grenzen, konstruktionsbedingt:**

- **Kein `PartialOrd`/keine Arithmetik über Skalen hinweg.**
  `Time<Gps> - Time<Glonass>` kompiliert nicht — das Vergleichen oder
  Subtrahieren über Skalen hinweg ohne explizite Konvertierung ist genau die
  Klasse von Bugs, die das Phantom-Typ-Design zur Compile-Zeit verhindern soll.
- **Keine automatische Aktualisierung der Schaltsekundentabelle.** Die
  eingebaute Tabelle ist zur Build-Zeit eingefroren; ein Empfänger, der die
  neuesten IERS-Ankündigungen benötigt, muss sein eigenes
  `RuntimeLeapSeconds` mitbringen (siehe `docs/LEAP_SECONDS.md` für die
  Update-Policy, die dies für die Crate-Maintainer impliziert, und
  `fuzz_try_extend.rs`/`fuzz_leap_lookup.rs` für den Vertrag, den die
  Laufzeittabelle erfüllen muss).
- **Schaltsekunden jenseits des letzten Tabelleneintrags werden als konstant
  angenommen.** `tai_minus_utc_at` gibt den letzten bekannten Offset für jeden
  TAI-Zeitpunkt nach dem letzten Tabelleneintrag zurück, gemäß IERS-Konvention
  (keine Schaltsekunde wird ohne ≥6 Monate Vorankündigung eingefügt) — das ist
  eine dokumentierte Annahme, kein Bug, aber es bedeutet, dass Konvertierungen
  für weit in der Zukunft liegende Daten stillschweigend „keine weiteren
  Schaltsekunden“ annehmen, statt einen Fehler zu werfen.
- **Keine Kalenderarithmetik auf `Time<S>` für GNSS-Skalen.** Nur `Time<Utc>`
  besitzt eine `to_civil()`/`CivilDateTime`-Ansicht; GPS/Galileo/BeiDou/GLONASS-
  Zeitstempel müssen zuerst nach UTC konvertiert werden, wenn ein
  Wanduhr-Datum benötigt wird — wiederum ein expliziter, fehlbarer Schritt
  statt eines impliziten.

## CI-Garantien

| Prüfung                         | Werkzeug                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------ |
| Kein unsicherer Code            | `#![forbid(unsafe_code)]`                                                                  |
| Keine undokumentierte API       | `#![deny(missing_docs)]`                                                                   |
| Build für Embedded-Ziele        | `cargo check --target thumbv7em-none-eabihf` (+ 4 weitere, siehe `docs/EMBEDDED.md`)       |
| Typgröße = 8 Bytes              | Unit-Test `test_size_equals_u64`                                                           |
| Sichere Arithmetik              | `-D warnings` + Abwesenheit von `#[allow(arithmetic_overflow)]`                            |
| Serde-Roundtrip (JSON)          | Tests in `src/serde_impls.rs`                                                              |
| Serde-Roundtrip (postcard)      | `tests/serde_test.rs`                                                                      |
| Schaltsekundentabelle wohlgeformt | `const`-Assertions in `tables/leap_seconds.rs`                                           |
| Vollständigkeit des Konvertierungsgraphen | erschöpfende paarweise Tests in `matrix.rs`                                      |
| Konvertierungs- & Lookup-Invarianten | Property-Tests (`tests/prop_*.rs`) + Fuzz-Harnesses (`fuzz/`, siehe `fuzz/README.md`) |
