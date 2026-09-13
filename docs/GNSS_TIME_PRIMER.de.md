# GNSS-Zeit-Grundlagen

Ein praktischer Leitfaden zu GNSS-Zeitsystemen für Softwareentwickler.

## Warum Zeit in GNSS schwierig ist

In den meisten Programmen ist Zeit ein einziges Konzept. Sie rufen `now()` auf,
erhalten eine Zahl, und diese Zahl bedeutet überall dasselbe.

In GNSS ist das nicht der Fall.

Jedes Satellitennavigationssystem führt seine eigene unabhängige Uhr. Diese
Uhren starten an unterschiedlichen Kalenderdaten, sind an unterschiedliche
Standards gebunden und akkumulieren im Laufe der Zeit unterschiedliche
ganzzahlige Offsets. Derselbe physikalische Zeitpunkt in der realen Welt hat
je nachdem, welches System Sie abfragen, einen anderen Zahlenwert.

Das ist keine Eigenheit, die man einfach „übertünchen“ kann. Ein Fehler
bedeutet, dass die Zeitstempel Ihres Empfängers um 14, 18 oder 19 Sekunden
abweichen — und Sie würden es nicht einmal bemerken.

## Vier Zeitskalen, die Sie verstehen müssen

### TAI — Internationale Atomzeit

TAI (Temps Atomique International) ist die Grundlage aller modernen
Zeitmessung.

- Wird über den gewichteten Mittelwert von ~450 Atomuhren weltweit geführt
- Enthält keine Schaltsekunden — eine absolut gleichmäßige Skala
- Epoche: **1958-01-01 00:00:00**
- Alle GNSS-Systeme sind über einen festen oder berechneten Offset an TAI gebunden

TAI ist die **Referenzskala** für alle Konvertierungen in dieser Bibliothek.
Jede Konvertierung zwischen GNSS-Skalen läuft intern über TAI.

### UTC — Koordinierte Weltzeit

UTC ist das, was Ihre Wanduhr anzeigt (ungefähr).

- UTC wird nahe an UT1 (Erdrotation) gehalten
- Verlangsamt sich die Erdrotation, wird eine **Schaltsekunde** eingefügt
- UTC = TAI − N, wobei N die Anzahl der eingefügten Schaltsekunden ist
- Seit 2017: **TAI − UTC = 37 Sekunden**

Schaltsekunden werden vom IERS im Voraus angekündigt (etwa 6 Monate im
Vorfeld). Es gibt keine Formel, um sie vorherzusagen — eine Tabelle ist
erforderlich.

### GLONASS-Zeit

GLONASS ist das russische Navigationssystem.

- Epoche: **1996-01-01 00:00:00 UTC(SU)** = 1995-12-31 21:00:00 UTC
- GLONASS führt **UTC(SU)** — die russische Realisierung von UTC, die UTC +
  3 Stunden (Moskauer Zeit) entspricht, **aber Schaltsekunden enthält**
- Da GLONASS mit UTC synchronisiert ist, ist die Konvertierung
  GLONASS ↔ UTC nur eine Epochenverschiebung: **+757 371 600 Sekunden**
- Für GLONASS ↔ UTC wird keine Schaltsekundentabelle benötigt
- Für GLONASS ↔ GPS / Galileo / BeiDou wird sie **jedoch** benötigt

### GPS-Zeit

GPS ist das amerikanische Navigationssystem.

- Epoche: **1980-01-06 00:00:00 UTC**
- Offset: **GPS = TAI − 19 Sekunden** (fest, keine Schaltsekunden)
- GPS war zu seiner Epoche mit UTC synchronisiert, als TAI − UTC = 19 s galt
- In GPS wurden nie Schaltsekunden eingefügt — die Zeit läuft kontinuierlich
- Seit 2017: **GPS liegt 18 Sekunden vor UTC**

GPS-Empfänger übertragen den aktuellen GPS–UTC-Offset in den
Navigationsnachrichten, damit die zivile Zeit berechnet werden kann.

### Galileo-Zeit

Galileo ist das europäische Navigationssystem.

- Epoche: **1999-08-22 00:00:00 UTC** (GPS-Woche 1024, TOW 0)
- Offset: **Galileo = TAI − 19 Sekunden** — identisch zu GPS
- Ein Galileo- und ein GPS-Nanosekundenwert mit demselben Wert
  **repräsentieren denselben physikalischen Zeitpunkt**
- Die Konvertierung GPS ↔ Galileo ist eine Identität: Der numerische
  Nanosekundenwert ändert sich nicht

### BDS-Zeit

BDT ist das chinesische Navigationssystem.

- Epoche: **2006-01-01 00:00:00 UTC**
- Offset: **BDT = TAI − 33 Sekunden**
- Da GPS = TAI − 19 s gilt, erhalten wir: **BDT = GPS − 14 Sekunden**
- Diese 14-Sekunden-Differenz ist fest und wird sich nie ändern

## Das Schaltsekunden-Problem

Schaltsekunden sind der schwierigste Teil der Arbeit mit GNSS-Zeit.

### Was ist eine Schaltsekunde?

Verlangsamt sich die Erdrotation, läuft UTC gegenüber UT1 (Sonnenzeit)
voraus. Um die Differenz innerhalb von 0,9 Sekunden zu halten, fügt der
IERS gelegentlich eine **positive Schaltsekunde** ein:

UTC-Uhren zeigen **23:59:60**, bevor sie auf **00:00:00** umspringen.

Negative Schaltsekunden (das Entfernen einer Sekunde) sind theoretisch
möglich, wurden aber noch nie angewendet.

### Wie GPS mit Schaltsekunden umgeht

GPS kennt keine Schaltsekunden. Die Zeit nimmt einfach zu.

Wird eine Schaltsekunde in UTC eingefügt, erhöht sich die GPS–UTC-Differenz um 1.

Vor dem 1981-07-01 betrug die Differenz 0 s. Nach dem 2017-01-01 beträgt sie
18 Sekunden.

| Ereignis    | TAI − UTC | GPS − UTC |
| ----------- | --------- | --------- |
| 1980-01-06  | 19 s      | 0 s       |
| 1981-07-01  | 20 s      | 1 s       |
| ...         | ...       | ...       |
| 1999-01-01  | 32 s      | 13 s      |
| 2017-01-01  | 37 s      | 18 s      |

GPS-Empfänger übertragen den aktuellen GPS–UTC-Offset (das `IODC`-Feld),
damit Software die zivile Zeit berechnen kann.

### Das 1-Sekunden-Ambiguitätsfenster

Zum Zeitpunkt einer Schaltsekunden-Einfügung gibt es ein 1-Sekunden-Fenster,
in dem derselbe GPS-Zeitstempel zwei UTC-Werten entspricht:

```zsh
GPS: 1_167_264_017 ns  →  UTC: 23:59:59  (letzte Sekunde vor der Schaltsekunde)
GPS: 1_167_264_018 ns  →  UTC: 23:59:60  (die eingefügte Schaltsekunde)
GPS: 1_167_264_018 ns  →  UTC: 00:00:00  (Beginn des nächsten Tages — derselbe GPS-Wert!)
```

Diese Bibliothek erkennt das Fenster und signalisiert es über
`ConvertResult::AmbiguousLeapSecond`.

## Konvertierungsgraph

```text
                    ┌──────────────────────────────────────────────┐
                    │           TAI (reference scale)              │
                    │  T_tai = T_self + OFFSET_TO_TAI              │
                    └──────┬──────┬───────┬──────┬─────────────────┘
                           │      │       │      │
               fixed +19s  │      │+19s   │+33s  │  contextual
                           ▼      ▼       ▼      │
                          GPS   Galileo  BeiDou  │
                           │      │       │      │
                           │ identity fixed      │
                           │                     ▼
                           │               UTC ←──── GLONASS
                           │               │  epoch shift
                           └───────────────┘
                            contextual (needs the leap-second table)
```

Feste Konvertierungen (keine Schaltsekunden erforderlich):

- GPS ↔ TAI, GPS ↔ Galileo, GPS ↔ BeiDou
- Galileo ↔ BeiDou, Galileo ↔ TAI, BeiDou ↔ TAI
- GLONASS ↔ UTC

Kontextabhängige Konvertierungen (ein `LeapSecondsProvider` ist erforderlich):

- GPS ↔ UTC, GPS ↔ GLONASS
- Galileo ↔ UTC, Galileo ↔ GLONASS
- BeiDou ↔ UTC, BeiDou ↔ GLONASS

## Häufige Fehler

### GPS als UTC verwenden

```rust
// FALSCH — GPS liegt nach 2017 18 Sekunden vor UTC
let gps = Time::<Gps>::from_seconds(gps_seconds_from_receiver);
let civil_time = gps.as_seconds(); // ← das sind GPS-Sekunden, nicht UTC!
```

```rust
// RICHTIG
let utc = gps.into_scale_with(LeapSeconds::builtin()).unwrap();
let civil_seconds = utc.as_seconds(); // UTC-Sekunden seit 1972-01-01
```

### Schaltsekunden bei GPS → UTC ignorieren

```rust
// FALSCH — nimmt feste 18 Sekunden an
let utc_seconds = gps.as_seconds() - 18;
```

```rust
// RICHTIG — verwendet die vollständige Schaltsekundentabelle
let utc = gps.into_scale_with(LeapSeconds::builtin()).unwrap();
```

### Zeitskalen in der Arithmetik mischen

```rust
// FALSCH — kompiliert in gnss-time nicht, aber ein häufiger Fehler:
// let delta = gps_time - glonass_time;
```

```rust
// RICHTIG — zunächst in eine einzige Skala umwandeln
let glo_as_utc: Time<Utc> = glonass.into_scale().unwrap();
let gps_as_utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
let delta = gps_as_utc - glo_as_utc;
```

## Quellen

- IS-GPS-200 Rev. N (2022) — GPS-Schnittstellenspezifikation
- ICD-GLONASS v5.1 (2008) — GLONASS-Schnittstellenkontrolldokument
- OS-SIS-ICD Issue 2.0 (2021) — Galileo-ICD
- BDS-SIS-ICD-B1I-3.0 (2019) — BeiDou-ICD
- IERS Bulletin C — Schaltsekunden-Ankündigungen: [https://www.iers.org](https://www.iers.org)
- Howard Hinnant, „Date Algorithms“: [http://howardhinnant.github.io/date_algorithms.html](http://howardhinnant.github.io/date_algorithms.html)
