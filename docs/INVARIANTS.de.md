# Invarianten und Sicherheitsgarantien

Dieses Dokument listet die Invarianten auf, die `gnss-time` einhält, dazu die
Formeln hinter ihnen sowie die Mechanismen — Typen, `const`-Assertions,
Unit-Tests, Property-Tests und Fuzz-Ziele — die jede einzelne durchsetzen.

Jede unten aufgeführte Invariante hat eine **Test**-Zeile. Wenn Sie eine
Invariante brechen, sollte einer dieser Tests fehlschlagen; tut er das nicht,
ist der Test der Bug.

## Inhaltsverzeichnis

- [Invarianten auf Typebene](#invarianten-auf-typebene)
- [Darstellungsinvarianten](#darstellungsinvarianten) — warum `u64` und nicht `i64`/`f64`
- [Arithmetische Invarianten](#arithmetische-invarianten) — Überlauf-Verhalten
- [Konvertierungsformeln](#konvertierungsformeln)
- [Roundtrip-Garantien](#roundtrip-garantien)
- [Das Schaltsekunden-Ambiguitätsfenster](#das-schaltsekunden-ambiguitätsfenster)
- [Speicherinvarianten](#speicherinvarianten)
- [Sicherheitsinvarianten](#sicherheitsinvarianten)
- [Invarianten-↔-Test-Querverweis](#invarianten--test-querverweis)

---

## Invarianten auf Typebene

### I-1: Domänenisolierung

`Time<A>`- und `Time<B>`-Werte (wobei `A ≠ B`) können nicht in arithmetischen
Ausdrücken gemischt werden.

**Durchsetzung:** das Rust-Typsystem. Die `Sub<Time<S>>`- und
`Add<Duration>`-Impls existieren nur für `Time<S>` mit demselben `S` (siehe
`src/time.rs`: `impl Add<Duration> for Time<S>`,
`impl Sub<Duration> for Time<S>`, `impl Sub<Time<S>> for Time<S>`). Jeder
Versuch, einen GLONASS-Zeitstempel von einem GPS-Zeitstempel zu subtrahieren,
führt zu einem Kompilierfehler.

**Test:** `examples/no_domain_mixing.rs` (ein `// does not compile`-Beispiel);
strukturell durchgesetzt — es gibt keinen Laufzeittest zu schreiben, weil die
Verletzung die Laufzeit gar nicht erreichen kann.

### I-2: Keine impliziten Konvertierungen

Es gibt keine `From`- / `Into`-Implementierungen zwischen verschiedenen
Skalen. Jede Konvertierung erfolgt explizit über einen Aufruf von
`into_scale()` oder `into_scale_with(ls)`. Siehe
`docs/ARCHITECTURE.de.md#grenzen` für die Begründung (Fehlbarkeit und der
benötigte Schaltsekunden-Kontext passen nicht zu `From`s infraillem Vertrag).

**Durchsetzung:** das Fehlen von Blanket-Implementierungen. Alle
`IntoScale`- / `IntoScaleWith`-Implementierungen sind von Hand geschrieben,
eine pro geordnetem Paar, und ihre Vollständigkeit wird durch die
erschöpfenden paarweisen Tests in `matrix.rs` geprüft.

### I-3: Versiegelte Zeitskalen

Externer Code kann `TimeScale` nicht implementieren. Die Menge der gültigen
Skalen ist: `{Gps, Glonass, Galileo, Beidou, Tai, Utc}`.

**Durchsetzung:** das `private::Sealed`-Supertrait-Muster (`src/scale.rs:39`).
Das `Sealed`-Trait lebt in einem privaten Modul und hat keinen öffentlichen
Pfad.

**Test:** `test_scale_types_are_copy` (`src/scale.rs:304`) und weitere in
`src/scale.rs` bestätigen, dass alle sechs Markertypen das Trait
implementieren; die Versiegelung selbst ist eine Compile-Time-Eigenschaft ohne
positiven Laufzeittest (eine mögliche Verletzung ist in Downstream-Code ein
Kompilierfehler, kein Testlauf in dieser Crate).

---

## Darstellungsinvarianten

### I-4: Nanosekunden werden als `u64` gespeichert, nicht als `i64` oder `f64`

`Time<S>` speichert `nanos: u64` — eine **vorzeichenlose** Anzahl von
Nanosekunden seit der Epoche von `S`. Dies ist eine bewusste Entscheidung mit
drei Teilen:

**Warum nicht vorzeichenbehaftet (`i64`)?** Eine `Time<S>` ist ein *Zeitpunkt*,
kein *Intervall* — negative Werte würden „vor der Epoche der Skala“
bedeuten, was keine unterstützte Skala darstellen muss (der nutzbare Bereich
jeder Skala beginnt per Definition bei oder nach ihrer eigenen Epoche). Die
Skala vorzeichenlos zu machen, verwandelt „Zeit, bevor diese Skala existierte“
in eine Typ-Ebene-Unmöglichkeit statt in eine Laufzeitprüfung:
`Time::<S>::EPOCH` (0 ns, `src/time.rs:128`) ist der kleinste darstellbare
Augenblick, Punkt. Deshalb können `checked_sub_duration` und der
`Sub`-Operator auch bei *positiven* Dauern nahe der Epoche fehlschlagen —
unter Null zu subtrahieren ist `Overflow`/Panik, kein negatives Ergebnis.

Im Gegensatz dazu ist `Duration` *vorzeichenbehaftet* (`i64`), gerade weil es
eine Differenz zwischen zwei Zeitpunkten repräsentiert und „früher als“
ausdrücken können muss — siehe [I-5](#i-5-duration-ist-vorzeichenbehaftet).

**Warum nicht Gleitkomma (`f64`)?** `f64` hat 52 Mantissenbits, genug für
exakte ganze Zahlen nur bis 2^53 ≈ 9,007 × 10^15. Ein Zeitstempel in
Nanosekunden-Auflösung erreicht diese Größenordnung in etwa 104 Tagen
(2^53 ns ≈ 104,25 Tage) — weit unter der nutzbaren Lebensdauer jeder
GNSS-Skala. Darüber hinaus rundet `f64` still auf den nächsten darstellbaren
Wert: zwei verschiedene Nanosekunden-Zeitpunkte könnten als gleich verglichen
werden, und Arithmetik wäre nicht-assoziativ, was die Roundtrip-Garantien
unten brechen würde. Ganzzahlige Nanosekunden haben keine dieser
Fehlerarten: jeder darstellbare `u64`-Wert ist exakt, und Vergleiche/
Arithmetik sind exakt, bis sie überlaufen — an diesem Punkt greift die
Überlauf-Politik der Crate (siehe
[I-9](#i-9-die-überlauf-politik-ist-explizit-und-einheitlich)), statt still
Präzision zu verlieren.

**Warum gerade 64 Bit?** `u64::MAX` Nanosekunden ≈ 584,5 Jahre — genug
Reserve jenseits der Epoche jeder Skala, sodass Überlauf ein echter
Randsonderfall ist (erreichbar nur nahe `Time::<S>::MAX`, gezielt ausgeübt von
den `fuzz_gps_utc`-/`fuzz_week_tow`-Fuzz-Zielen im Grenzwertmodus) und kein
Alltagsproblem. Ein `u32`-Zähler (≈ 4,29 Sekunden Nanosekunden-Auflösung)
wäre nutzlos; `u128` würde die Typgröße ohne Nutzen innerhalb der praktischen
Lebensdauer jeder Skala verdoppeln.

**Test:** `test_size_equals_u64` (`src/time.rs:1131`, bestätigt das 8-Byte-,
`u64`-identische Layout); das `f64`-Präzisionsverlust-Argument oben ist
strukturell und nicht laufzeitgetestet (es gibt in der Crate keinen
`f64`-gestützten Alternativtyp zum Vergleich).

### I-5: `Duration` ist vorzeichenbehaftet

`Duration` verwendet `i64`-Nanosekunden (`src/duration.rs:94`,
`#[repr(transparent)]`). Die Subtraktion eines späteren Zeitpunkts von einem
früheren ergibt eine negative `Duration`, und sie lässt sich zu *jedem* der
beiden Operanden addieren, um den anderen wiederzugewinnen. Dies ist das
Intervall-Gegenstück zu [I-4](#i-4-nanosekunden-werden-als-u64-gespeichert-nicht-als-i64-oder-f64):
eine `Time<S>` ist ein Punkt (`u64`, konstruktionsbedingt vorzeichenlos),
eine `Duration` eine Verschiebung zwischen zwei Punkten (`i64`,
notwendigerweise vorzeichenbehaftet).

**Test:** `test_sub_times_negative` (`src/time.rs:1280`),
`test_negative` (`src/duration.rs:592`).

---

## Arithmetische Invarianten

### I-6: Kein stiller Überlauf

Die `+`- und `-`-Operatoren für `Time<S>` und `Duration` **paniken** bei
Überlauf. Das ist beabsichtigt, kein Versehen — siehe
[I-9](#i-9-die-überlauf-politik-ist-explizit-und-einheitlich) für die
vollständige Politik und warum Paniken die *Standard*- statt der *einzigen*
Option ist.

**Durchsetzung:** Wrapping-Arithmetik wird nie verwendet. Die
`-D warnings`-Flags in CI (`RUSTFLAGS` in `.github/workflows/ci.yml`)
escalieren den per Default deny-`arithmetic_overflow`-Lint, der bei
Compile-Zeit-Konstanten-Überlauf greift, sodass jeder solche Überlauf den
Haupt-Build fehlschlagen lässt; die panikenden Operator-Implementierungen
fangen jeden *Laufzeit*-Überlauf ab. `#[allow(arithmetic_overflow)]` ist in
der Crate überall verboten.

**Test:** `test_add_operator_panics_at_max` (`src/time.rs:1671`),
`test_sub_operator_panics_at_epoch` (`src/time.rs:1677`);
`#[should_panic]`-Tests in `src/duration.rs`.

### I-7: `u64::MAX` ist die harte Obergrenze; `EPOCH` ist die harte Untergrenze

`Time::<S>::MAX.as_nanos() == u64::MAX` und
`Time::<S>::MIN == Time::<S>::EPOCH == Time::from_nanos(0)`
(`src/time.rs:134`). Keine Operation kann einen `Time<S>`-Wert außerhalb von
`[EPOCH, MAX]` erzeugen; jeder arithmetische Pfad panikt, gibt `None` zurück,
sättigt auf eine der beiden Grenzen oder gibt `Err(Overflow)` zurück — siehe
[I-9](#i-9-die-überlauf-politik-ist-explizit-und-einheitlich).

**Durchsetzung:** alle Arithmetik läuft in `i128` mit einer expliziten
Bereichsprüfung gegen `[0, u64::MAX]` vor dem Rückcast auf `u64`
(`to_tai`, `from_tai`, `try_convert`, `checked_add`,
`checked_sub_duration`, `checked_elapsed` in `src/time.rs`); der Cast selbst
kann nicht still wrappen, weil die Bereichsprüfung zuerst erfolgt.

**Test:** `test_max_is_u64_max` (`src/time.rs:1548`),
`test_checked_add_at_max_overflows` (`src/time.rs:1583`),
`test_checked_sub_at_epoch_underflows` (`src/time.rs:1605`);
der RAW-Modus von `fuzz_week_tow`/`fuzz_day_tod` übt die gesamte
Eingabedomäne der Konstruktoren aus, die `Time<S>`-Werte erzeugen, sodass jeder
Pfad, der `[EPOCH, MAX]` verlassen könnte, dort als
Assertion-Fehler aufscheinen würde.

### I-8: `checked_elapsed` bringt die Differenz in `i64` unter

`Time<S>::checked_elapsed(earlier)` (`src/time.rs:374`) berechnet
`self − earlier` als `Duration` (`i64`-Nanosekunden) und gibt `None` zurück,
wenn die wahre Differenz nicht in `i64` passt — was möglich ist, weil `Time<S>`
den gesamten `u64`-Bereich (≈ 584,5 Jahre) überspannt, während `Duration` nur
`i64` (≈ ±292 Jahre) überspannt: die Lücke zwischen `Time::<S>::MIN` und
`Time::<S>::MAX` ist genau doppelt so groß, wie eine einzelne `Duration`
ausdrücken kann.

**Test:** `test_checked_elapsed_overflows_when_gap_exceeds_i64` (`src/time.rs:1689`),
`test_checked_elapsed_within_i64_range_works` (`src/time.rs:1698`).

### I-9: Die Überlauf-Politik ist explizit und einheitlich

Jede fehlbare arithmetische Operation auf `Time<S>` und `Duration` wird in
(bis zu) vier Formen angeboten, und die Crate mischt sie nie still — der
*Name* der Methode verrät den Fehlermodus:

| Suffix / Form           | Bei Überlauf…                                         | Verwenden Sie, wenn…                                                                                                      |
| ----------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `+` / `-` (Operatoren)  | **Panik**                                             | Überlauf ein Programmierfehler ist, den Sie sofort fangen wollen (Standard; entspricht der `std`-Konvention)               |
| `checked_*`             | gibt **`None`** zurück                                | Sie auf Fehler verzweigen möchten, ohne einen Fehlertyp zu allozieren                                                     |
| `saturating_*`          | **klemmt** auf `EPOCH`/`MAX` (bzw. `Duration::MIN`/`MAX`) | Embedded-/Regelschleifen, in denen eine Panik schlimmer wäre als ein geklemmter Wert                                        |
| `try_*`                 | gibt **`Err(GnssTimeError::Overflow)`** zurück        | Sie den Fehler in eine `Result`-basierte Aufrufkette integrieren möchten (`?`)                                             |

Keine Operation wrappt still (Zweierkomplement-Überlauf) oder verliert still
Präzision — das sind die beiden Fehlermodi, die von
[I-6](#i-6-kein-stiller-überlauf) bzw.
[I-4](#i-4-nanosekunden-werden-als-u64-gespeichert-nicht-als-i64-oder-f64)
explizit ausgeschlossen sind. Die vier Formen oben sind die *vollständige*
Menge legaler Verhaltensweisen; wenn Sie eine fünfte finden (z. B. eine
Methode, die statt einer dieser vier einen falschen-aber-gültigen Wert
zurückgibt), ist das ein Bug.

**Test:** `test_time_max_behavior` (`src/time.rs:1533`) übt alle drei
nicht-panikenden Formen (`checked_add`, `saturating_add`, `try_add`) gegen
dieselbe überlaufende Eingabe und prüft, dass jede das dokumentierte Ergebnis
liefert; gepaart mit den `#[should_panic]`-Tests in
[I-6](#i-6-kein-stiller-überlauf) für die Operator-Form.

---

## Konvertierungsformeln

### I-10: TAI ist der universelle Dreh- und Angelpunkt für Skalen mit festem Offset

```text
T_tai = T_self + S::OFFSET_TO_TAI
```

Diese Gleichung gilt für jede Skala mit `OffsetToTai::Fixed` (`Gps`,
`Galileo`, `Beidou`, `Tai` selbst mit Offset 0). Alle paarweisen
Konvertierungen zwischen solchen Skalen werden aus dieser einen Formel
abgeleitet, die zweimal zusammengesetzt wird (`to_tai()` dann `from_tai()`,
d. h. `try_convert::<T>()` — `src/time.rs:275`) — es gibt keinen
paarweisen Spezialfall.

**Durchsetzung:** `try_convert<T>` ruft `to_tai()` und dann `T::from_tai()`
auf. Keine Fest-Offset-Konvertierung umgeht TAI.

**Test:** `test_into_scale_gps_tai_matches_to_tai` (`src/convert.rs:916`);
`test_roundtrip_via_tai` (`src/time.rs:1339`).

### I-11: Formeln mit festem Offset, Skala für Skala

| Konvertierung          | Formel                                                                          | Verwendeter fester Offset                                                     |
| ---------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| GPS → TAI              | `T_tai = T_gps + 19 s`                                                          | `Gps::OFFSET_TO_TAI`                                                          |
| Galileo → TAI          | `T_tai = T_gal + 19 s`                                                          | `Galileo::OFFSET_TO_TAI`                                                      |
| BeiDou → TAI           | `T_tai = T_bdt + 33 s`                                                          | `Beidou::OFFSET_TO_TAI`                                                       |
| GPS ↔ Galileo          | `T_gal.as_nanos() = T_gps.as_nanos()` (Identität — siehe [I-14](#i-14-gps-galileo-identität)) | beide `+19 s`                                              |
| GPS ↔ BeiDou           | `T_bdt = T_gps − 14 s`                                                          | `19 s − 33 s = −14 s`                                                          |
| Galileo ↔ BeiDou       | `T_bdt = T_gal − 14 s`                                                          | wie GPS↔BeiDou (via TAI)                                                      |
| GLONASS ↔ UTC          | `T_utc = T_glo + 757 371 600 s`                                                 | Epochenverschiebung, **nicht** via TAI — siehe [I-13](#i-13-glonass-utc-epochenoffset) |

Jede Zeile außer GLONASS↔UTC ist eine direkte Konsequenz von
[I-10](#i-10-tai-ist-der-universelle-dreh--und-angelpunkt-für-skalen-mit-festem-offset);
GLONASS↔UTC ist `IntoScale` (fest, kein Schaltsekunden-Kontext nötig), geht
aber nicht durch TAI, weil GLONASS' `OffsetToTai` `Contextual` ist, nicht
`Fixed` — die GLONASS↔UTC-Beziehung ist *relativ zu UTC* fest, nicht relativ
zu TAI.

**Test:** für jede Zeile existiert ein Test in `src/convert.rs`
(`test_gps_to_tai_adds_19_seconds`, `src/convert.rs:632`;
`test_gps_to_beidou_subtracts_14_seconds`, `src/convert.rs:707`;
`test_glonass_epoch_to_utc_nanos`, `src/convert.rs:749`; …) und in
`src/time.rs` (`test_roundtrip_via_tai`, `src/time.rs:1339`) — siehe die
[Querverweis-Tabelle](#invarianten--test-querverweis) für die vollständige
Liste.

### I-12: Kontextformel — GPS → UTC und der Zwei-Pass-Algorithmus UTC → GPS

```text
UTC_ns_from_1972 = GPS_ns_from_1980 − (TAI_minus_UTC(t) − 19) × 1e9
                    + UTC_TO_GPS_EPOCH_NS
```

wobei `TAI_minus_UTC(t)` aus dem `LeapSecondsProvider` zu dem TAI-Augenblick
nachgeschlagen wird, der der Eingabe-GPS-Zeit entspricht, und
`UTC_TO_GPS_EPOCH_NS = 252 892 800 × 1e9` (`src/leap.rs:82`) die konstante
Verschiebung zwischen der UTC-Epoche (1972-01-01) und der GPS-Epoche
(1980-01-06) ist. Das `−19` zieht den Offset ab, der bereits in GPS↔TAI
eingebacken ist, und lässt nur die *seit* der GPS-Epoche akkumulierten
Schaltsekunden übrig. Dieselbe Konstante erscheint in den Code-Kommentaren der
Formel und wird per `const`-Assertion als exakt 252 892 800 s (2927 Tage)
verifiziert.

Die Rückrichtung `utc_to_gps` verwendet einen **Zwei-Pass**-Algorithmus
(`src/leap.rs:952`): der erste Durchgang berechnet TAI näherungsweise unter
Annahme `GPS − UTC = 0`, der zweite Durchgang verfeinert das Ergebnis mit der
im ersten Durchgang gefundenen Schaltsekundenanzahl. Das macht die
Konvertierung an allen 18 Schaltsekunden-Grenzen der GPS-Ära korrekt.

**Test:** `test_gps_leads_utc_by_18s_at_2017_01_01` (`src/convert.rs:828`)
und `test_gps_leads_utc_by_13s_at_1999_01_01` (`src/convert.rs:843`) fixieren
die Formel an zwei Übergangsdaten; `tests/roundtrip_test.rs::test_all_gps_era_leap_second_transitions`
deckt dieselbe Formel an **allen 18** Übergängen ab, ebenso die
`BOUNDARY_SECONDS`-Liste in `tests/prop_tests.rs:282`;
der BOUNDARY-Modus von `fuzz_gps_utc.rs` übt diese Formel an allen 18
historischen Übergängen plus Jitter aus.

### I-13: GLONASS-UTC-Epochenoffset

Die GLONASS-Epoche = 1995-12-31 21:00:00 UTC = 757 371 600 Sekunden ab der
UTC-Epoche (1972-01-01). Dies ist eine Compile-Time-Konstante, verifiziert wie
folgt (`src/leap.rs:68`):

```rust
const _VERIFY_GLONASS_OFFSET: () = {
    assert!(GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000 == 757_371_600);
};
```

**Test:** `test_glonass_epoch_offset_is_757371600_seconds` (`src/leap.rs:1072`);
`test_glonass_epoch_offset_from_utc_epoch_is_correct` (`src/leap.rs:1087`)
kreuzprüft dieselbe Konstante gegen unabhängige `CivilDate`-Arithmetik.

### I-14: GPS-Galileo-Identität

GPS und Galileo haben *denselben* festen Offset,
`OFFSET_TO_TAI = 19 000 000 000 ns` (`src/scale.rs:169`/`:183`). Für denselben
physikalischen Zeitpunkt gilt daher `T_gps.as_nanos() == T_gal.as_nanos()` —
die Konvertierung ist ein Typwechsel ohne Arithmetik (`ConversionKind::Identity`
in `src/matrix.rs:25`).

**Test:** `test_gps_galileo_identity_via_tai` (`src/time.rs:1347`);
`test_gps_galileo_is_identity` (`src/matrix.rs:318`).

---

## Roundtrip-Garantien

### I-15: Wann gilt `A → B → A == A`?

| Konvertierungsklasse                                                                                                   | Roundtrip-Garantie                                                                                                                                                                                                                              |
| ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Identity` (GPS ↔ Galileo)                                                                                             | **Immer exakt.** Es findet keine Arithmetik statt; die Nanosekundenanzahl ist unverändert.                                                                                                                                                      |
| `Fixed` (GPS ↔ TAI/BeiDou, Galileo ↔ BeiDou/TAI)                                                                       | **Immer exakt** für jeden Wert, der in keine Richtung überläuft. Das `to_tai`/`from_tai`-Paar ist eine reine Ganzzahl-Addition/-Subtraktion ohne Rundung.                                                                                        |
| `EpochShift` (GLONASS ↔ UTC)                                                                                           | **Immer exakt** für jeden Wert, der nicht überläuft (insbesondere können UTC-Werte vor der GLONASS-Epoche *nicht in* GLONASS hinein roundtrippen — sie fehlschlagen mit `Overflow`, nicht still).                                                |
| `Contextual`, **außerhalb** des Schaltsekunden-Fensters (GPS ↔ UTC, GPS ↔ GLONASS und ihre Galileo-/BeiDou-Äquivalente) | **Exakt.** `gps_to_utc(utc_to_gps(t, ls), ls) == t` für jedes `t`, für das `into_scale_with_checked` `ConvertResult::Exact` meldet.                                                                                                              |
| `Contextual`, **innerhalb** des Schaltsekunden-Fensters                                                               | **Nicht garantiert exakt** — siehe [I-16](#i-16-wann-kann-gps--utc-ambiguousleapsecond-erzeugen). `into_scale_with_checked` meldet `ConvertResult::AmbiguousLeapSecond` genau dafür, damit Aufrufer diesen Fall erkennen statt einem möglicherweise-um-eine-Sekunde-falschen Ergebnis zu vertrauen. |

Die präzise Grenze für „außerhalb des Fensters“ in der kontextuellen Zeile ist
exakt die Menge der von `into_scale_with_checked` als `Exact` klassifizierten
Augenblicke — das ist *per Definition* so, keine Näherung davon, denn
`ConvertResult` existiert genau dazu, diese Grenze abfragbar statt
erschlossen zu machen.

**Test:** `test_gps_utc_gps_roundtrip_at_gps_epoch` (`src/leap.rs:1295`),
`test_gps_utc_gps_roundtrip_at_2020` (`src/leap.rs:1305`),
`test_gps_utc_roundtrip_exact_at_nanosecond_level` (`src/convert.rs:817`);
`prop_gps_utc_gps_roundtrip_exact` in `tests/prop_tests.rs:127` (256
Stichprobenpunkte, Ambiguitätsfenster ausgeschlossen); `fuzz_gps_utc.rs` /
`fuzz_utc_to_gps.rs` mit Invariante **I-12** in der Nummerierung dieser
Harnesse (Roundtrip-Genauigkeit — siehe die Doc-Kommentare der Harnesse)
prüft die Exaktheit auf dem `Exact`-Zweig und eine ≤1s-Schranke auf dem
`AmbiguousLeapSecond`-Zweig für die *gesamte* `u64`-Domäne, nicht nur für
Stichprobenpunkte.

---

## Das Schaltsekunden-Ambiguitätsfenster

### I-16: Wann kann `GPS → UTC` `AmbiguousLeapSecond` erzeugen?

Eine Schaltsekunden-Einfügung fügt eine extra UTC-Sekunde (23:59:60) hinzu,
die kein GPS-Gegenstück hat — GPS-Zeit wiederholt oder überspringt
konstruktionsbedingt nie eine Sekunde. Konkret ist die Abbildung von
GPS-Nanosekunden auf UTC-Nanosekunden um jede der 18 historischen
Einfügungen herum für genau das Ein-Sekunden-Fenster unmittelbar *vor* der
Einfügung **nicht injektiv**: sowohl die letzte reguläre GPS-Sekunde als auch
die folgende Schalt-GPS-Sekunde kommen als Kandidaten für dieselbe
UTC-Bezeichnung in Frage, je nachdem, von welcher Seite des
Einfüge-Augenblicks der UTC-Wert aus interpretiert wird.

`into_scale_with_checked` erkennt dies, indem es `TAI − UTC` am angefragten
Augenblick mit `TAI − UTC` eine Sekunde früher vergleicht (`src/convert.rs:530`):

```text
n_now    = tai_minus_utc_at(tai)
n_before = tai_minus_utc_at(tai − 1s)

n_now != n_before  ⇒  ConvertResult::AmbiguousLeapSecond
n_now == n_before  ⇒  ConvertResult::Exact
```

Das heißt: das ambigue Fenster ist **exakt** eine Sekunde breit, verankert an
jeder Übergangsschwelle im `LeapSecondsProvider` (alle 18 in der integrierten
Tabelle) — nie breiter, nie schmaler und nie sonst irgendwo vorhanden.

**Was Aufrufer tun sollten:** `gps_to_utc`/`utc_to_gps` (die freien
Funktionen und `into_scale_with`) geben auch im Fenster immer eine einzelne
best-effort-`Time<Target>` zurück — sie verweigern nie die Antwort. Verwenden
Sie `into_scale_with_checked`, wenn die Unterscheidung wichtig ist (z. B.
Logging, Auditing oder alles, wo eine 1-Sekunden-Diskrepanz an einer
Schaltsekunden-Grenze ein Korrektheitsproblem für den Aufrufer wäre), und
inspizieren Sie die `ConvertResult`-Variante.

**Test:** `test_gps_to_utc_detects_leap_second_ambiguity` (`src/convert.rs:936`),
`test_leap_second_transition_1999_gps_jumps_by_2s` (`src/leap.rs:1375`),
`test_leap_second_transition_2017_gps_jumps_by_2s` (`src/leap.rs:1400`);
`prop_ambiguous_only_near_boundaries` in `tests/prop_tests.rs:311` prüft dies
programmatisch für alle 18 Übergänge statt nur der zwei handverlesenen oben;
`prop_gps_near_leap_converts_consistently` (`tests/prop_tests.rs:336`) prüft
jeden Übergang; der BOUNDARY-Modus von `fuzz_gps_utc.rs`/`fuzz_utc_to_gps.rs`
ist speziell so konstruiert, dass er in jedem dieser Fenster in jedem
Fuzz-Lauf landet (siehe die Doc-Kommentare dieser Harnesse dafür, warum eine
blinde `u64`-Mutation ein 2 Sekunden breites Ziel in einem Raum von ≈1,8×10^19
Punkten nicht zuverlässig erreichen kann).

---

## Speicherinvarianten

### I-17: Keine Heap-Allokation

`Time<S>` und `Duration` sind `Copy`-Typen ohne `Drop`-Implementierung.
`LeapSeconds::builtin()` gibt ein `&'static LeapSeconds` zurück, das auf ein
statisches Array zeigt (`src/leap.rs:50`). `RuntimeLeapSeconds` ist ein
Puffer fester Größe auf dem Stack/statisch (`[LeapEntry; RUNTIME_CAPACITY]`,
`src/leap.rs:226`), kein `Vec`. Das `alloc`-Crate wird nirgendwo im eigenen
Code der Crate verwendet, mit oder ohne das `serde`-Feature (siehe
`docs/ARCHITECTURE.de.md#serde-unterstützung-feature--serde`).

**Durchsetzung:** `#![no_std]` in `src/lib.rs:55` ohne `extern crate alloc`.

**Test:** `test_no_heap_allocation_in_conversions` in
`tests/no_std_compact.rs:193`; der CI-Job `no-std-transitive` in
`.github/workflows/embedded.yml` durchsucht den Abhängigkeitsbaum in jeder
Feature-Kombination nach `std`.

### I-18: 8-Byte-Größe

`size_of::<Time<S>>() == 8` für alle `S: TimeScale`, und `size_of::<Duration>()
== 8`.

**Durchsetzung:** das Layout ist `{ nanos: u64, _scale: PhantomData<S> }` mit
`PhantomData`, das null Bytes beiträgt; `Duration` ist `#[repr(transparent)]`
über `i64` (`src/duration.rs:94`).

**Test:** `test_size_equals_u64` (`src/time.rs:1131`) läuft in der Standard-
Testsuite und wird für das Embedded-Ziel im `type-sizes`-Job in
`.github/workflows/embedded.yml` erneut verifiziert; die `firmware/`-
Größenprobe-Crate bestätigt zusätzlich, dass die *kompilierte* Darstellung
übereinstimmt (keine versteckte Polsterung durch eine spezifische Ziel-ABI).

---

## Sicherheitsinvarianten

### I-19: Kein Unsafe-Code

`#![forbid(unsafe_code)]` in `src/lib.rs:56`. Jeder Versuch, Unsafe-Code
hinzuzufügen, ist ein Kompilierfehler, keine Warnung oder ein lokal
unterdrückbarer Lint.

**Test:** `#![forbid(...)]` (vs. `#![deny(...)]`) kann nicht durch ein inneres
`#[allow(unsafe_code)]` übersteuert werden, sodass dies bei *jedem* Build von
`rustc` selbst durchgesetzt wird, nicht von einem CI-Grep.

### I-20: Keine fehlende Dokumentation

`#![deny(missing_docs)]` in `src/lib.rs:57`. Jedes öffentliche Element muss
eine Dokumentation haben.

**Test:** `#![deny(missing_docs)]` wird bei jedem Build von
`rustc`/`cargo doc` durchgesetzt (und im CI-`lint`-Job per Grep verifiziert);
`cargo clippy --all-targets -- -D warnings` läuft im `clippy`-CI-Job für alle
Feature-Kombinationen.

---

## Invarianten-↔-Test-Querverweis

Schneller Nachschlag von der Invariante zu den Tests, die sie fixieren,
gruppiert nach Quelle. Diese Tabelle ist der inverse Index der
**Test**-Zeilen pro Invariante oben — verwenden Sie sie, wenn ein Test
fehlschlägt und Sie wissen müssen, *welche* Invariante er schützt, oder wenn
Sie eine Formel ändern und jede Stelle wissen müssen, die davon abhängt.

| Invariante                             | Unit-Tests                                                                                                                                     | Property-/Fuzz-Tests                                                                                                        |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| I-1 Domänenisolierung                  | `examples/no_domain_mixing.rs` (Compile-Fail)                                                                                                  | —                                                                                                                            |
| I-2 Keine impliziten Konvertierungen   | erschöpfende `impl`-Revision in den `matrix.rs`-Tests                                                                                          | —                                                                                                                            |
| I-3 Versiegelte Skalen                 | `src/scale.rs::test_scale_types_are_copy`                                                                                                      | —                                                                                                                            |
| I-4 `u64`-Darstellung                  | `src/time.rs::test_size_equals_u64`                                                                                                            | —                                                                                                                            |
| I-5 `Duration` vorzeichenbehaftet      | `src/duration.rs::test_negative`, `src/time.rs::test_sub_times_negative`                                                                       | —                                                                                                                            |
| I-6 Kein stiller Überlauf              | `src/time.rs::test_add_operator_panics_at_max`, `src/duration.rs`-`#[should_panic]`-Tests                                                        | —                                                                                                                            |
| I-7 `[EPOCH, MAX]`-Grenze              | `src/time.rs::test_max_is_u64_max`, `::test_checked_add_at_max_overflows`, `::test_checked_sub_at_epoch_underflows`                            | `fuzz/fuzz_targets/fuzz_week_tow.rs`, `fuzz_day_tod.rs` (RAW-Modus)                                                          |
| I-8 `checked_elapsed` in `i64`         | `src/time.rs::test_checked_elapsed_overflows_when_gap_exceeds_i64`, `::test_checked_elapsed_within_i64_range_works`                            | —                                                                                                                            |
| I-9 Einheitliche Überlauf-Politik      | `src/time.rs::test_time_max_behavior`                                                                                                          | —                                                                                                                            |
| I-10 TAI als Dreh- und Angelpunkt      | `src/convert.rs::test_into_scale_gps_tai_matches_to_tai`, `src/time.rs::test_roundtrip_via_tai`                                                | —                                                                                                                            |
| I-11 Formeln pro Skala                 | `src/convert.rs::test_gps_to_tai_adds_19_seconds`, `::test_gps_to_beidou_subtracts_14_seconds`, `::test_glonass_epoch_to_utc_nanos`            | —                                                                                                                            |
| I-12 GPS→UTC-Formel + Zwei-Pass        | `src/convert.rs::test_gps_leads_utc_by_18s_at_2017_01_01`, `::test_gps_leads_utc_by_13s_at_1999_01_01`, `tests/roundtrip_test.rs::test_all_gps_era_leap_second_transitions` | `tests/prop_tests.rs::BOUNDARY_SECONDS`; BOUNDARY-Modus von `fuzz/fuzz_targets/fuzz_gps_utc.rs`             |
| I-13 GLONASS-UTC-Offset                | `src/leap.rs::test_glonass_epoch_offset_is_757371600_seconds`, `::test_glonass_epoch_offset_from_utc_epoch_is_correct`                         | —                                                                                                                            |
| I-14 GPS-Galileo-Identität             | `src/time.rs::test_gps_galileo_identity_via_tai`, `src/matrix.rs::test_gps_galileo_is_identity`                                               | —                                                                                                                            |
| I-15 Roundtrip-Garantien               | `src/leap.rs::test_gps_utc_gps_roundtrip_at_gps_epoch`, `::test_gps_utc_gps_roundtrip_at_2020`, `src/convert.rs::test_gps_utc_roundtrip_exact_at_nanosecond_level` | `tests/prop_tests.rs::prop_gps_utc_gps_roundtrip_exact`; `fuzz_gps_utc.rs`/`fuzz_utc_to_gps.rs`              |
| I-16 Schaltsekunden-Fenster            | `src/convert.rs::test_gps_to_utc_detects_leap_second_ambiguity`, `src/leap.rs::test_leap_second_transition_1999_gps_jumps_by_2s`, `::test_leap_second_transition_2017_gps_jumps_by_2s` | `tests/prop_tests.rs::prop_ambiguous_only_near_boundaries`, `::prop_gps_near_leap_converts_consistently`; BOUNDARY-Modus von `fuzz_gps_utc.rs`/`fuzz_utc_to_gps.rs` |
| I-17 Keine Heap-Allokation             | `tests/no_std_compact.rs::test_no_heap_allocation_in_conversions`                                                                              | CI: Job `no-std-transitive` in `.github/workflows/embedded.yml`                                                              |
| I-18 8-Byte-Größe                      | `src/time.rs::test_size_equals_u64`                                                                                                            | CI: Job `type-sizes`; Größenprobe `firmware/`                                                                                 |
| I-19 Kein Unsafe-Code                  | `rustc` (`#![forbid(unsafe_code)]`)                                                                                                            | —                                                                                                                            |
| I-20 Keine fehlende Dokumentation      | `rustc` (`#![deny(missing_docs)]`), `cargo clippy --all-targets -- -D warnings`                                                                | —                                                                                                                            |

Zusätzliche reine Fuzz-Abdeckung, die keiner einzelnen nummerierten Invariante
oben zugeordnet ist:

- **`fuzz_try_extend.rs`** — fixiert den vollständigen Fehlerprioritäts-Vertrag
  von `RuntimeLeapSeconds::try_extend` (`BufferFull` >
  `NotStrictlyAscending`/`NonUnitIncrement`/`OffsetOverflow`) gegen ein
  unabhängiges `i64`-Referenzmodell; dies ist die Harness, die den
  `i32::MAX`-Offset-Wraparound-Fix (`OffsetOverflow`) gefunden und fixiert
  hat, dokumentiert in `CHANGELOG.md`.
- **`fuzz_leap_lookup.rs`** — Monotonie und dynamischer Bereich (`min..=max`
  der *tatsächlichen* Tabelle, nicht hartkodiert `19..=37`) von
  `tai_minus_utc_at` für sowohl `LeapSeconds` als auch `RuntimeLeapSeconds`,
  einschließlich des Fallbacks bei leerer Tabelle.
- **`fuzz_week_tow.rs`/`fuzz_day_tod.rs`** — exakte Fehlerklassifizierung
  (`InvalidInput` vs. `Overflow` vs. `Ok`) und Feld-Roundtrip-Exaktheit für
  `from_week_tow`/`from_day_tod` über ihrer vollständigen Eingabedomäne.

Siehe `fuzz/README.md` dafür, wie diese lokal ausgeführt werden und den
aktuellen Null-Crash-Status.
