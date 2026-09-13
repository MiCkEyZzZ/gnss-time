# Schaltsekunden

Dokumentation der eingebauten Schaltsekunden-Tabelle in `gnss-time`, der
Aktualisierungsrichtlinie und der Erweiterung zur Laufzeit.

## Datenquelle

Alle Daten stammen aus dem IERS Bulletin C:
<https://hpiers.obspm.fr/iers/bul/bulc/Leap_Second.dat>

**Letzte Verifizierung:** IERS Bulletin C 70 (Dezember 2024) — bis Juni 2025
sind keine neuen Schaltsekunden geplant.
**Aktueller Stand (Mai 2026):** TAI − UTC = 37 s, unverändert seit 2017-01-01.

## Was eine Schaltsekunde ist

TAI (Internationale Atomzeit) ist eine kontinuierliche atomare Zeitskala.
UTC ist die Zivilzeit, die durch regelmäßiges Einfügen von **Schaltsekunden**
innerhalb von 0,9 s um UT1 (astronomische Zeit) gehalten wird.

Beim Einfügen „stoppt“ UTC für eine Sekunde — der Moment `23:59:60 UTC`
existiert, während GPS kontinuierlich weiterläuft. Die Differenz `TAI − UTC`
erhöht sich um 1.

GPS fügt niemals Sekunden ein und überspringt keine: `GPS = TAI − 19 s`
(fest seit 1980-01-06). Daher erfordert eine Konvertierung GPS ↔ UTC die
Kenntnis der aktuellen Differenz `TAI − UTC`.

## Tabellenformat

Jeder Eintrag ist `(tai_nanos_threshold, tai_minus_utc)`:

- `tai_nanos_threshold` — die TAI-Nanosekunden (GPS-relativ), ab denen der
  neue Wert wirksam wird (inklusiv, untere Grenze)
- `tai_minus_utc` — der Wert `TAI − UTC` in ganzen Sekunden ab diesem
  Schwellenwert

### Formel zur Berechnung der Schwellenwerte

```text
tai_nanos = (unix_event_timestamp − GPS_EPOCH_UNIX + tai_minus_utc) × 10⁹
```

wobei `GPS_EPOCH_UNIX = 315 964 800` (Sekunden der Unix-Zeit).

Beispiel für 2017-01-01 (`unix = 1 483 228 800`, `n = 37`):

```text
gps_s     = 1_483_228_800 − 315_964_800 = 1_167_264_000
threshold = (1_167_264_000 + 37) × 10⁹  = 1_167_264_037_000_000_000
```

## Vollständige Tabelle (19 Einträge, GPS-Ära)

| #   | Ereignisdatum | TAI−UTC | GPS−UTC | tai_nanos-Schwellenwert   |
| --- | ------------- | ------- | ------- | ------------------------- |
| 0   | 1980-01-06    | 19      | 0       | 0                         |
| 1   | 1981-07-01    | 20      | 1       | 46 828 820 000 000 000    |
| 2   | 1982-07-01    | 21      | 2       | 78 364 821 000 000 000    |
| 3   | 1983-07-01    | 22      | 3       | 109 900 822 000 000 000   |
| 4   | 1985-07-01    | 23      | 4       | 173 059 223 000 000 000   |
| 5   | 1988-01-01    | 24      | 5       | 252 028 824 000 000 000   |
| 6   | 1990-01-01    | 25      | 6       | 315 187 225 000 000 000   |
| 7   | 1991-01-01    | 26      | 7       | 346 723 226 000 000 000   |
| 8   | 1992-07-01    | 27      | 8       | 393 984 027 000 000 000   |
| 9   | 1993-07-01    | 28      | 9       | 425 520 028 000 000 000   |
| 10  | 1994-07-01    | 29      | 10      | 457 056 029 000 000 000   |
| 11  | 1996-01-01    | 30      | 11      | 504 489 630 000 000 000   |
| 12  | 1997-07-01    | 31      | 12      | 551 750 431 000 000 000   |
| 13  | 1999-01-01    | 32      | 13      | 599 184 032 000 000 000   |
| 14  | 2006-01-01    | 33      | 14      | 820 108 833 000 000 000   |
| 15  | 2009-01-01    | 34      | 15      | 914 803 234 000 000 000   |
| 16  | 2012-07-01    | 35      | 16      | 1 025 136 035 000 000 000 |
| 17  | 2015-07-01    | 36      | 17      | 1 119 744 036 000 000 000 |
| 18  | 2017-01-01    | 37      | 18      | 1 167 264 037 000 000 000 |

## Aktualisierungsrichtlinie der Tabelle

Der IERS veröffentlicht das Bulletin C zweimal jährlich (Januar und Juli).
Jede Ausgabe berichtet:

- ob in den nächsten 6 Monaten eine neue Schaltsekunde eingefügt wird, oder
- bestätigt, dass es keine Änderungen gibt.

### Wann aktualisiert werden sollte

Wenn der IERS eine neue Schaltsekunde ankündigt:

1. **Schwellenwert berechnen** mit der obigen Formel und dem
   Unix-Zeitstempel des Ereignisses.
2. **Eintrag hinzufügen** am Ende des `BUILTIN_TABLE`-Arrays in
   `src/tables/leap_seconds.rs`:

   ```rust
   // JJJJ-MM-TT: TAI−UTC → N
   LeapEntry::new(<threshold>, <N>),
   ```

3. **Den `// Last verified:`-Kommentar** in derselben Datei aktualisieren.
4. **Den Kopf dieses Dokuments** (`LEAP_SECONDS.de.md`) aktualisieren.
5. **Die Tests ausführen** — die Compile-Time-Assertions in `leap_seconds.rs`
   verifizieren Ordnung und Monotonie automatisch:

   ```bash
   cargo test
   cargo check --target thumbv7em-none-eabihf
   ```

6. **`CHANGELOG.md` aktualisieren** und eine Patch-Version des Crates
   veröffentlichen.

### Automatische Korrektheitsprüfungen der Tabelle

In `src/tables/leap_seconds.rs` werden drei `const`-Assertions definiert, die
bereits während der **Kompilierung** ausgelöst werden (nicht nur zur
Testzeit):

| Assertion                 | Was geprüft wird                                              |
| ------------------------- | ------------------------------------------------------------- |
| `_ASSERT_FIRST_ENTRY`     | `tai_nanos == 0`, `tai_minus_utc == 19`                       |
| `_ASSERT_TABLE_INVARIANTS`| strikte Ordnung und ein +1-Inkrement über die gesamte Tabelle |
| `_ASSERT_LAST_ENTRY`      | der letzte Eintrag entspricht 2017-01-01, `n == 37`           |

Wenn Sie einen Eintrag mit falschem Schwellenwert hinzufügen oder ein
Inkrement überspringen, lehnt der Compiler dies **sofort** ab — ohne Tests
auszuführen.

> **Wichtig:** beim Hinzufügen eines neuen Eintrags müssen Sie
> `_ASSERT_LAST_ENTRY` aktualisieren — ändern Sie die erwarteten Werte für
> `tai_nanos` und `tai_minus_utc`.

## Verwendung der eingebauten Tabelle

```rust
use gnss_time::{LeapSeconds, gps_to_utc, LeapSecondsProvider};
use gnss_time::{Time, Gps, Tai};

// Eingebaute Tabelle: deckt alle Ereignisse der GPS-Ära bis 2017-01-01 ab
let ls = LeapSeconds::builtin();

// Diagnose: Wann fand das letzte Ereignis statt?
let last = ls.last_update().unwrap();

assert_eq!(last.as_nanos(), 1_167_264_037_000_000_000); // 2017-01-01

// Aktueller TAI−UTC-Wert
assert_eq!(ls.current_tai_minus_utc(), 37);

// Konvertierung GPS → UTC
let gps = Time::<Gps>::from_seconds(1_167_264_018); // 2017-01-01 GPS
let utc = gps_to_utc(gps, &ls).unwrap();
```

## Aktualisierung zur Laufzeit für Embedded / Empfänger

Für GNSS-Empfänger, die die Tabelle aus einer Navigationsnachricht oder aus
dem Almanach laden, verwenden Sie `RuntimeLeapSeconds`:

```rust
use gnss_time::{LeapEntry, RuntimeLeapSeconds, LeapSecondsProvider};

// Ausgehend vom Compile-Time-Snapshot der eingebauten Tabelle
let mut rt = RuntimeLeapSeconds::from_builtin();

// Der Empfänger meldet eine neue Sekunde aus dem Almanach
// (hypothetisches Ereignis — nur zu Illustrationszwecken)
// rt.try_extend(LeapEntry::new(threshold_ns, 38)).unwrap();

// Wird an denselben Stellen wie LeapSeconds::builtin() verwendet
let gps = gnss_time::Time::<gnss_time::Gps>::from_seconds(1_000_000);
let utc = gnss_time::gps_to_utc(gps, &rt).unwrap();
```

### API von `RuntimeLeapSeconds`

| Methode                    | Beschreibung                                        |
| -------------------------- | --------------------------------------------------- |
| `from_builtin()`           | Erstellt eine Tabelle aus dem Compile-Time-Snapshot |
| `from_slice(&[LeapEntry])` | Erstellt aus einem beliebigen Slice                 |
| `try_extend(entry)`        | Fügt einen neuen Eintrag mit Validierung hinzu      |
| `last_update()`            | TAI-Moment des letzten Ereignisses                  |
| `current_tai_minus_utc()`  | Aktueller TAI−UTC-Wert                              |
| `len()` / `is_empty()`     | Tabellengröße                                       |
| `entries()`                | Alle Einträge als Slice                             |

### Validierung bei `try_extend`

`try_extend` lehnt ungültige Einträge ab:

```rust
use gnss_time::{LeapEntry, LeapExtendError, RuntimeLeapSeconds};

let mut rt = RuntimeLeapSeconds::from_builtin();

// Fehler: der Schwellenwert steigt nicht strikt an
let err = rt.try_extend(LeapEntry::new(0, 38)).unwrap_err();

assert_eq!(err, LeapExtendError::NotStrictlyAscending);

// Fehler: das Inkrement ist nicht 1
let err = rt.try_extend(LeapEntry::new(9_999_999_999_000_000_000, 99)).unwrap_err();

assert_eq!(err, LeapExtendError::NonUnitIncrement);
```

## Eigener Provider

Für volle Kontrolle implementieren Sie das `LeapSecondsProvider`-Trait:

```rust
use gnss_time::{LeapSecondsProvider, Tai, Time};

/// Provider mit festem Wert (zum Beispiel für Tests).
struct FixedOffset(i32);

impl LeapSecondsProvider for FixedOffset {
    fn tai_minus_utc_at(&self, _tai: Time<Tai>) -> i32 {
        self.0
    }
}

let provider = FixedOffset(37);
```

## Künftige Schaltsekunden und ihre Abschaffung

Die Tabelle enthält keine Einträge nach 2017-01-01. Für spätere Zeitpunkte
liefert die Bibliothek `TAI − UTC = 37` (den letzten bekannten Wert) zurück —
der übliche Ansatz „annehmen, dass keine neuen Schaltsekunden kommen“.

**Stand der Abschaffung:** Auf der Weltfunkkonferenz 2022 (WRC-22) wurde
beschlossen, die Einfügung von Schaltsekunden **spätestens bis 2035**
einzustellen. Bis zu dieser Frist bleibt die aktuelle Tabelle gültig. Nach
2035 wird die Konvertierung UTC ↔ GPS vollständig deterministisch (so wie
heute GPS ↔ TAI).

### IERS-Beobachtung

Zur Verfolgung neuer Ankündigungen:

- **Bulletin C:** <https://hpiers.obspm.fr/iers/bul/bulc/bulletinC.dat>
- **Aktuelle Datendatei:** <https://hpiers.obspm.fr/iers/bul/bulc/Leap_Second.dat>
- **IETF LEAPSECOND:** <https://www.ietf.org/timezones/data/leap-seconds.list>
