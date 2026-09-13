# Invarianten und Sicherheitsgarantien

Dieses Dokument listet die Invarianten auf, die `gnss-time` einhält, zusammen
mit den Mechanismen, die sie durchsetzen.

## Invarianten auf Typebene

### I-1: Domänenisolierung

`Time<A>`- und `Time<B>`-Werte (wobei `A ≠ B`) können nicht in arithmetischen
Ausdrücken gemischt werden.

**Durchsetzung:** das Rust-Typsystem. Die `Sub<Time<S>>`- und
`Add<Duration>`-Impls existieren nur für `Time<S>` mit demselben `S`. Jeder
Versuch, einen GLONASS-Zeitstempel von einem GPS-Zeitstempel zu subtrahieren,
führt zu einem Kompilierfehler.

### I-2: Keine impliziten Konvertierungen

Es gibt keine `From`- / `Into`-Implementierungen zwischen verschiedenen Skalen.
Jede Konvertierung erfolgt explizit über einen Aufruf von `into_scale()` oder
`into_scale_with(ls)`.

**Durchsetzung:** das Fehlen von Blanket-Implementierungen. Alle `IntoScale`- /
`IntoScaleWith`-Implementierungen sind von Hand geschrieben und verifiziert.

### I-3: Versiegelte Zeitskalen

Externer Code kann `TimeScale` nicht implementieren. Die Menge der gültigen
Skalen ist: `{Gps, Glonass, Galileo, Beidou, Tai, Utc}`.

**Durchsetzung:** das `private::Sealed`-Supertrait-Muster. Das `Sealed`-Trait
lebt in einem privaten Modul und hat keinen öffentlichen Pfad.

## Arithmetische Invarianten

### I-4: Kein stiller Überlauf

Alle `+`- und `-`-Operatoren für `Time<S>` und `Duration` lösen bei Überlauf
eine Panik aus. Für Code, bei dem Panik nicht akzeptabel ist, werden checked-,
saturating- und fehlbare Varianten bereitgestellt.

**Durchsetzung:** Wrapping-Arithmetik wird nicht verwendet. Der
`#[deny(arithmetic_overflow)]`-Lint (über `-D warnings` in CI) fängt
versehentliche Überläufe bereits zur Compile-Zeit ab.
`#[allow(arithmetic_overflow)]` ist in CI verboten.

### I-5: `u64::MAX` ist die harte Obergrenze

`Time::<S>::MAX.as_nanos() == u64::MAX`. Keine Operation kann einen größeren
Wert erzeugen; stattdessen wird entweder gepanikt, `None` zurückgegeben,
saturiert oder `Err` zurückgegeben.

**Durchsetzung:** die gesamte Arithmetik erfolgt in `i128` mit einer
Bereichsprüfung vor dem Rückcast auf `u64`.

### I-6: `Duration` ist vorzeichenbehaftet

`Duration` verwendet `i64`-Nanosekunden. Die Subtraktion eines späteren
Zeitpunkts von einem früheren ergibt eine negative `Duration`. Dadurch lässt
sich in beide Richtungen natürlich arbeiten.

## Konvertierungsinvarianten

### I-7: TAI ist der universelle Dreh- und Angelpunkt

```text
T_tai = T_self + S::OFFSET_TO_TAI
```

Diese Gleichung gilt für alle Skalen mit festem Offset (`Gps`, `Galileo`,
`Beidou`, `Tai`). Alle paarweisen Konvertierungen werden aus dieser Formel
abgeleitet. Es gibt keine „Magie“ für bestimmte Skalenpaare.

**Durchsetzung:** `try_convert<T>` ruft `to_tai()` und dann `T::from_tai()`
auf. Keine Konvertierung umgeht TAI.

### I-8: GPS–Galileo-Identität

GPS und Galileo haben denselben Offset
`OFFSET_TO_TAI = 19_000_000_000 ns`. Daher gilt
`T_gps.as_nanos() == T_gal.as_nanos()` für denselben physikalischen Zeitpunkt.

**Test:** `test_gps_galileo_identity_via_tai` in `src/time.rs`.

### I-9: Fester GPS–BeiDou-Offset

`BDT = GPS − 14s` immer. Das folgt aus `GPS+19 = BDT+33 = TAI`.

**Test:** `test_gps_to_beidou_subtracts_14_seconds` in `src/time.rs`.

### I-10: GLONASS–UTC-Epochenoffset

Die GLONASS-Epoche = 1995-12-31 21:00:00 UTC = 757 371 600 Sekunden ab der
UTC-Epoche (1972-01-01). Dies ist eine Compile-Time-Konstante, verifiziert wie
folgt:

```rust
const _VERIFY_GLONASS_OFFSET: () = {
    assert!(GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000 == 757_371_600);
};
```

### I-11: Korrektheit des Zwei-Pass-Algorithmus UTC → GPS

Der Zwei-Pass-Algorithmus `utc_to_gps` ist an allen 18 Schaltsekunden-Grenzen
der GPS-Ära korrekt.
**Tests:** `prop_all_18_leap_second_transitions_correct` in
`tests/prop_tests.rs` sowie die einzelnen Übergangstests in `src/leap.rs`.

### I-12: Roundtrip-Genauigkeit außerhalb des Ambiguitätsfensters

Für jedes `t: Time<Gps>`, das **nicht** in das 1-Sekunden-Schaltsekunden-
Ambiguitätsfenster fällt:
`gps_to_utc(utc_to_gps(t, ls), ls) == t`.

**Test:** `prop_gps_utc_gps_roundtrip_for_all_samples` in
`tests/prop_tests.rs` (256 Punkte; das Ambiguitätsfenster wird über
`AmbiguousLeapSecond` übersprungen).

## Speicherinvarianten

### I-13: Keine Heap-Allokation

`Time<S>` und `Duration` sind `Copy`-Typen ohne `Drop`-Implementierung.
`LeapSeconds::builtin()` gibt ein `&'static LeapSeconds` zurück, das auf ein
statisches Array zeigt. Das `alloc`-Crate wird nirgendwo verwendet.

**Durchsetzung:** `#![no_std]` in `lib.rs` ohne `extern crate alloc`. Der Test
`all_conversions_are_stack_only` in `tests/no_std_compat.rs`.

### I-14: 8-Byte-Größe

`size_of::<Time<S>>() == 8` für alle `S: TimeScale`.

**Durchsetzung:** der Unit-Test `test_size_equals_u64` läuft in CI über den
`type-sizes`-Job in `.github/workflows/embedded.yml`.

## Sicherheitsinvarianten

### I-15: Kein Unsafe-Code

`#![forbid(unsafe_code)]` in `lib.rs`. Jeder Versuch, Unsafe-Code
hinzuzufügen, ist ein Kompilierfehler, keine Warnung.

**CI-Prüfung:** `grep -n "forbid(unsafe_code)" src/lib.rs` im `lint`-Job.

### I-16: Keine fehlende Dokumentation

`#![deny(missing_docs)]` in `lib.rs`. Jedes öffentliche Element muss eine
Dokumentation haben.

**CI-Prüfung:** `cargo clippy -- -D warnings` im `lint`-Job.