# API-Stabilität

Dieses Dokument ist die einzige maßgebliche Quelle dafür, worauf man sich in
`gnss-time` heute sicher verlassen kann, was sich vor `1.0` noch ändern darf
und was niemals direkt benannt werden sollte. Es existiert, weil
`#[non_exhaustive]` und Doc-Kommentare Stabilität *pro Element* vermitteln,
aber nirgendwo bisher die Politik *als Ganzes* formuliert war oder jedes
öffentliche Element im Verhältnis zu ihr aufgelistet wurde.

## Inhaltsverzeichnis

- [Semver-Politik für `0.x`](#semver-politik-für-0x)
- [Stabilitätslegende](#stabilitätslegende)
- [Audit: Abdeckung von `#[non_exhaustive]`](#audit-abdeckung-von-non_exhaustive)
- [API-Matrix](#api-matrix)
- [Re-Export-Audit für `pub use`](#re-export-audit-für-pub-use)
- [Interne Elemente, die `#[doc(hidden)]` erhalten sollten](#interne-elemente-die-dochidden-erhalten-sollten)
- [Abdeckung der Doc-Tests](#abdeckung-der-doc-tests)
- [Durchsetzung in CI](#durchsetzung-in-ci)

---

## Semver-Politik für `0.x`

`gnss-time` ist vor `1.0` (`0.x`). Cargos Semver-Kompatibilitätsregel für `0.x`
behandelt **die Minor-Version als Grenze für brechende Änderungen**: `0.5.2` →
`0.6.0` darf brechen; `0.5.1` → `0.5.2` darf es nicht. Diese Crate folgt dieser
Regel strikt, mit einer Ergänzung, die für den Aufbau der Crate spezifisch ist:

| Änderung                                                                                              | Bump in `0.x`                                                                                                                                                                              |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Neue Variante zu einem `#[non_exhaustive]`-enum hinzugefügt                                           | **patch**                                                                                                                                                                                  |
| Neue Methode an einem bestehenden Typ hinzugefügt                                                     | **patch**                                                                                                                                                                                  |
| Neues Feld in einer `#[non_exhaustive]`-Struktur hinzugefügt                                          | **patch**                                                                                                                                                                                  |
| Neues Trait-`impl` für einen bestehenden Typ                                                          | **patch**                                                                                                                                                                                  |
| Ein Element wird als veraltet markiert (`#[deprecated]`, Element bleibt erhalten und funktioniert)              | **patch**                                                                                                                                                                                  |
| Neuer öffentlicher `#[non_exhaustive]`-Typ oder neue freie Funktion                                         | **minor**                                                                                                                                                                                  |
| Neuer Eintrag in der eingebauten Schaltsekunden-Tabelle (`tables/leap_seconds.rs`)                    | **patch** — Begründung unten                                                                                                                                                               |
| Entfernen eines `#[deprecated]`-Elements                                                              | **minor**                                                                                                                                                                                  |
| Ändern der Signatur einer öffentlichen Funktion (einschließlich ihres Fehlertyps)                     | **minor**                                                                                                                                                                                  |
| Ändern des `Display`-Ausgabeformats eines öffentlichen Typs                                           | **minor**                                                                                                                                                                                  |
| Hinzufügen einer Variante zu einem **nicht**-`#[non_exhaustive]`-enum                                  | **minor** (wäre sonst eine brechende Änderung für fremde `match`)                                                                                                                           |
| Vergrößern eines numerischen Parametertyps (z.B. `u16` → `u32`, wie bei `from_week_tow` in `#TIME-27.1`) | **minor** — Aufrufstellen können brechen, selbst wenn die Änderung „permissiver" ist, weil Typinferenz und overload-artige generische Grenzen anders auswählen können                    |
| Verengen eines numerischen Parametertyps oder Ändern eines Rückgabetyps                                | **major** (oder minor vor `1.0`, aber mit Prüfung auf major-Ebene)                                                                                                                          |
| Ändern der Konstante `TimeScale::OFFSET_TO_TAI` für eine bestehende Skala                             | **major** — das ist eine Korrektheitskonstante, keine API-Form; sie leise zu ändern ändert das *numerische Ergebnis* jeder nachgelagerten Konvertierung, was ein schwerwiegenderer Bruch als ein Kompilierfehler ist |
| Erhöhung der MSRV (`rust-version` in `Cargo.toml`)                                                  | **minor** — ein Konsument, der an eine ältere Toolchain gebunden ist, kann die Crate dann überhaupt nicht mehr kompilieren; aus seiner Sicht ist das nicht von einer brechenden Änderung zu unterscheiden       |
| Senkung der MSRV                                                                                     | **patch** — rein additiv: alles, was vorher kompilierte, kompiliert weiter                                                                                                                                    |

**Warum ist ein neuer Schaltsekunden-Tabelleneintrag ein *patch* und kein
Minor-Bump?** Eine Schaltsekunden-Einfügung wird von der IERS angekündigt; es
handelt sich um externe, faktische Daten — sie ändert weder die Signatur einer
Funktion, den Fehlertyp noch die Menge der Typen, die ein Aufrufer benennen
kann. Sie ändert nur den *numerischen Output* von
`gps_to_utc`/`utc_to_gps` für Zeitpunkte nach dem neuen Eintrag — genau so, wie
es die eigene Dokumentation des Crates ankündigt (siehe die
Aktualisierungsrichtlinie in `docs/LEAP_SECONDS.de.md`). Sie als patch zu
behandeln bedeutet, dass Nutzer das korrigierte Schaltsekunden-Verhalten über
ein normales `cargo update` erhalten, was das beabsichtigte Verhalten ist — die
Alternative (jede Schaltsekunde als Minor-Bump zu behandeln) würde Nutzer dazu
erziehen, `gnss-time = "=0.5.2"` festzupinnen und unbegrenzt mit veralteten
Schaltsekunden-Daten zu arbeiten.

**Warum ist das Erweitern eines Parametertyps besonders markiert?** `#TIME-27.1`
änderte den Parameter `week` von `Time::<Gps>::from_week_tow` von `u16` auf
`u32`, passend zu `week() -> u32`. Für direkte Aufrufer ist das *normalerweise*
sicher (`u16`-Werte konvertieren weiterhin), aber es ist nicht universell
nicht-brechend: Ein Aufrufer, der `from_week_tow(2345u16, …)` geschrieben hat,
kompiliert weiterhin (ganzzahlige Literale sind untypisiert), aber ein Aufrufer
mit `let w: u16 = ...; from_week_tow(w, ...)` braucht jetzt eine explizite
`w as u32`-Konvertierung (oder `.into()`), und jeder Aufruf durch
einen generischen Wrapper wie `impl Into<u32>` verändert seinen abgeleiteten Typ.
Einen Kompatibilitäts-Alias `from_week_tow_u16` gibt es nicht (ein früherer
Entwurf dieses Dokuments behauptete dessen Existenz); die Erweiterung ist eine
regelrechte brechende Änderung, festgehalten unter `[0.7.0]` in
`CHANGELOG.md`.

**Kriterien für `1.0` (noch nicht erfüllt):** jedes unten mit **Stabil bis
1.0** markierte Element hat einen vollständigen `0.x`-Zyklus ohne gemeldete
Signatur- oder Semantikänderungen durchlaufen, die Fuzz-Suite (`fuzz/README.md`)
läuft seit mindestens einem Release-Zyklus kontinuierlich in CI ohne
Abstürze, und alle Invarianten aus `docs/INVARIANTS.de.md` sind mit einem
bestehenden Test verknüpft.

---

## Stabilitätslegende

| Badge                 | Bedeutung                                                                                                                                                                                                      |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 🟢 **Stabil**         | Kann heute als Abhängigkeit genutzt werden. Ändert sich nur über die `0.x`-Regeln oben (d.h. für Signatur/Semantik wie post-`1.0` behandelt; darf laut Patch-Bump-Tabelle weiterhin um `#[non_exhaustive]`-Elemente ergänzt werden). |
| 🟡 **Stabil bis 1.0** | Design ist festgelegt und durch Fuzzing-/Property-Tests geprüft, hat aber noch keinen vollständigen Release-Zyklus unverändert durchlaufen. Soll bei `1.0` unverändert 🟢 werden.                             |
| 🟠 **Instabil**       | Die API kann sich noch ändern. Benutzbar, aber pinnen Sie eine konkrete `0.x.y`-Version, wenn man auf die exakte Signatur angewiesen ist.                                                                     |
| 🔴 **Intern**         | Kein Teil des öffentlichen Vertrags, auch wenn es gerade erreichbar ist. Nicht darauf verlassen; siehe [`#[doc(hidden)]`-Empfehlungen](#interne-elemente-die-dochidden-erhalten-sollten).               |

---

## Audit: Abdeckung von `#[non_exhaustive]`

Alle öffentlichen enums und alle öffentlichen Strukturen, die für zukünftige
Felderweiterungen gedacht sind, wurden geprüft. Ergebnis:

| Typ                  | `#[non_exhaustive]`?                       | Urteil                                                                                                                                                                                                                                   |
| -------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GnssTimeError`      | ✅ ja                                       | Korrekt — neue Fehlervarianten sind eine patch-Ebene-Ergänzung.                                                                                                                                                                            |
| `LeapExtendError`    | ✅ ja                                       | Korrekt — die Fehlermenge von `try_extend` kann wachsen (z.B. eine künftige `OffsetOverflow`-artige Variante, gefunden durch Fuzzing, `CHANGELOG.md`).                                                                                    |
| `ConvertResult<T>`   | ✅ ja                                       | Korrekt — derzeit `Exact`/`AmbiguousLeapSecond`; eine dritte Variante (z.B. zur Unterscheidung, auf welcher Seite der Schaltsekunde das Ergebnis liegt) ist denkbar.                                                                    |
| `ConversionKind`     | ✅ ja                                       | Korrekt — das Klassifikations-enum von `matrix.rs`; eine neue Art wäre nötig, wenn eine Skala mit einer wirklich neuen Beziehung hinzukäme.                                                                                               |
| `ScaleId`            | ✅ ja                                       | Korrekt — enum der sechs Skalen zur Laufzeit-Introspektion; eine neue Skala ergibt eine neue Variante, dank des Attributs patch-sicher.                                                                                                  |
| `DisplayStyle`       | ✅ ja                                       | Korrekt — intern für die assoziierte Konstante `TimeScale::DISPLAY_STYLE` in `scale.rs`; neue Anzeigestile sind plausibel (z.B. ein `CivilLike`-Stil, wenn eine andere Skala eine Kalenderansicht bekommt).                                 |
| `OffsetToTai`        | ✅ ja                                       | Korrekt — derzeit `Fixed(i64)`/`Contextual`; entspricht der Fixed/Contextual-Trennung in `docs/ARCHITECTURE.de.md`.                                                                                                                     |
| `DurationParts`      | ❌ nein (einfache Struktur, zwei öffentliche Felder) | **Bewusste Ausnahme** — siehe unten.                                                                                                                                                                                                     |
| `CivilDateTime`      | ❌ nein (einfache Struktur, sieben öffentliche Felder) | **Bewusste Ausnahme** — siehe unten.                                                                                                                                                                                                     |
| `Time<S>`            | n/a (ein privates Feld + `PhantomData`)     | Korrekt — es gibt ohnehin keine öffentlichen Felder; der gesamte Zugriff erfolgt über Methoden.                                                                                                                                           |
| `Duration`           | n/a (ein privates Feld)                     | Korrekt — gleiche Begründung.                                                                                                                                                                                                            |
| `LeapEntry`          | ❌ nein (einfache Struktur, zwei öffentliche Felder) | **Markiert — siehe Empfehlung unten.**                                                                                                                                                                                                    |
| `LeapSeconds`        | n/a (undurchsichtig, privates Feld / private Felder) | Korrekt.                                                                                                                                                                                                                               |
| `RuntimeLeapSeconds` | n/a (undurchsichtig, privates Feld / private Felder) | Korrekt.                                                                                                                                                                                                                               |

**`DurationParts` und `CivilDateTime` sind bewusst *nicht*
`#[non_exhaustive]`.** Beide sind einfache Datenstrukturen, deren
ganzer Zweck darin besteht, mit einem Struktur-Literal konstruiert zu werden
(`DurationParts { seconds, nanos }`, `CivilDateTime { year, month, … }`) — sie
als `#[non_exhaustive]` zu markieren würde jeden Aufrufer zu `::new()` oder
`Default` mit anschließender Mutation zwingen und den ergonomischen Zweck des Typs zunichte
machen. Ein Feld zu einer von beiden in Zukunft hinzuzufügen wäre ein **minor**-
Bump laut Tabelle oben, wie jede brechende Struktur-Literal-Änderung; das wird
als Kosten dafür akzeptiert, dass die Literal-Konstruktion verfügbar bleibt.

**`LeapEntry` ist für einen `0.x`-Minor-Fix markiert**: Es ist eine einfache
Zwei-Feld-Struktur (`tai_nanos: u64, tai_minus_utc: i32`), die direkt von
Aufrufern konstruiert wird, die eine eigene Schaltsekunden-Tabelle für
`RuntimeLeapSeconds`/`LeapSeconds::try_from_slice` aufbauen. Anders als bei
`DurationParts`/`CivilDateTime` gibt es hier keinen starken ergonomischen Grund,
ein Literal zu bevorzugen — `LeapEntry::new(tai_nanos, tai_minus_utc)` ist
genauso lesbar wie ein Literal, und jede Aufrufstelle in den aktuellen Test- und
Fuzz-Suiten verwendet bereits den Konstruktor mit zwei Argumenten.
**Empfehlung:** `#[non_exhaustive]` im nächsten Minor-Release setzen, da ein
künftiges Feld (z.B. ein menschenlesbares Ankündigungsdatum oder ein Quellen-/
Provenienz-Tag, das IERS-Tabelleneinträge von empfängergelieferten
unterscheidet) plausibel und derzeit durch das exponierte Literal blockiert ist.

---

## API-Matrix

Nach Modul gruppiert, in derselben Reihenfolge wie das Schichten-Diagramm in
`docs/ARCHITECTURE.de.md`.

### `scale` (Schicht 1)

| Element                                                            | Stabilität             | Anmerkungen                                                                                                                                                                                                                                 |
| ------------------------------------------------------------------ | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `TimeScale` (Trait)                                                | 🟡 Stabil bis 1.0    | Versiegelt; siehe `docs/ARCHITECTURE.de.md#das-sealed-trait-muster`. Externe `impl`-Blöcke sind strukturell unmöglich, daher betrifft seine „Stabilität" wirklich den Vertrag der assoziierten Konstanten, nicht die Erweiterbarkeit.                |
| `Gps`, `Glonass`, `Galileo`, `Beidou`, `Tai`, `Utc` (Markertypen)  | 🟢 Stabil              | Zero-Size, `Copy`; das Hinzufügen eines *neuen* Markertyps ist additiv (minor-Bump) und betrifft diese sechs nie.                                                                                                                |
| `OffsetToTai`                                                      | 🟡 Stabil bis 1.0    | `#[non_exhaustive]`; siehe Audit oben.                                                                                                                                                                                                      |
| `DisplayStyle`                                                     | 🟠 Instabil            | `#[non_exhaustive]`, wird derzeit aber nur intern durch `Display for Time<S>` verwendet — siehe [`#[doc(hidden)]`-Empfehlung](#interne-elemente-die-dochidden-erhalten-sollten) unten; die öffentliche Offenlegung könnte nicht beabsichtigt gewesen sein. |

### `epoch` (Schicht 1)

| Element                                                                                                        | Stabilität             | Anmerkungen                                                                                                                                                                             |
| -------------------------------------------------------------------------------------------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CivilDate`                                                                                                    | 🟡 Stabil bis 1.0    | Einfache Struktur; wird sowohl für die Epochen-Definitionen der Skalen als auch (indirekt) in den Algorithmen von `CivilDateTime` verwendet.                                              |
| `TAI_EPOCH`, `UNIX_EPOCH`, `UTC_CIVIL_EPOCH`, `GPS_EPOCH`, `GLONASS_EPOCH`, `GALILEO_EPOCH`, `BEIDOU_EPOCH`  | 🟢 Stabil              | Compile-Zeit-Konstanten, verifiziert durch `docs/INVARIANTS.de.md`.                                                                                                                      |
| `UTC_EPOCH_UNIX_OFFSET_S`, `UTC_EPOCH_UNIX_OFFSET_NS`, `GPS_EPOCH_UNIX_S`                                     | 🟢 Stabil              | Aus `prelude` re-exportiert; siehe [Re-Export-Audit](#re-export-audit-für-pub-use).                                                                                                      |
| Konstanten `LEAP_SECONDS_AT_*_EPOCH`, `DAYS_GPS_TO_*`, `NANOS_GPS_TO_*`                                       | 🟠 Instabil            | Öffentlich, aber nicht über `prelude` re-exportiert; vermutlich als interne Ableitungskonstanten gedacht statt als Teil der primären API. Kandidaten für `#[doc(hidden)]` — siehe unten. |

### `time` (Schicht 2)

| Element                                                                                                                                 | Stabilität             | Anmerkungen                                                                                                                                                                                                                                   |
| --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Time<S>`                                                                                                                               | 🟡 Stabil bis 1.0    | Zentraler Werttyp. Das 8-Byte-Layout ist eine [Invariante](INVARIANTS.de.md#i-18-8-byte-größe), nicht nur Dokumentation.                                                                                                                       |
| `Time::<S>::EPOCH`, `MIN`, `MAX`, `NANOS_PER_YEAR`                                                                                      | 🟢 Stabil              | Assoziierte Konstanten.                                                                                                                                                                                                                       |
| `from_nanos`, `from_seconds`, `checked_from_seconds`, `as_nanos`, `as_seconds`, `as_seconds_f64`                                        | 🟢 Stabil              | Konstruktoren und Accessoren für alle `S`.                                                                                                                                                                                                        |
| `to_tai`, `from_tai`, `try_convert`                                                                                                     | 🟡 Stabil bis 1.0    | Der TAI-Pivot-Mechanismus selbst ([I-10](INVARIANTS.de.md#i-10-tai-ist-der-universelle-pivot-für-skalen-mit-festem-offset)); dürfte seine Form nicht ändern, hat aber noch keinen vollständigen Zyklus durchlaufen.                          |
| `checked_add`, `checked_sub_duration`, `saturating_add`, `saturating_sub_duration`, `try_add`, `try_sub_duration`, `checked_elapsed`    | 🟢 Stabil              | Die Vier-Formen-Überlauf-Politik ist als Invariante dokumentiert ([I-9](INVARIANTS.de.md#i-9-die-überlauf-politik-ist-explizit-und-einheitlich)); den *Namen* einer Form zu ändern wäre ein minor-Bump, aber die *Politik* selbst ist fixiert. |
| `Add<Duration>`, `Sub<Duration>`, `Sub<Time<S>>`, `AddAssign`, `SubAssign`, `Ord`, `PartialOrd`, `Debug`, `Display`                     | 🟢 Stabil              | Standard-Trait-Impls.                                                                                                                                                                                                                         |
| `DurationParts`                                                                                                                         | 🟢 Stabil              | Warum es eine einfache Struktur bleibt — siehe `#[non_exhaustive]`-Audit oben.                                                                                                                                                                |
| `Time::<Glonass>::from_day_tod`, `day`, `tod_seconds`, `sub_second_nanos`, `day_of_week`, `is_weekend`                                  | 🟢 Stabil              | GLONASS-spezifische Accessoren.                                                                                                                                                                                                               |
| `Time::<Gps>::from_week_tow`, `week`, `tow_seconds`, `sub_second_nanos`                                                                 | 🟢 Stabil              | Signatur-Geschichte (die `u16`→`u32`-Erweiterung) steht in der Semver-Tabelle oben.                                                                                                                                                           |
| `Time::<Gps>::from_unix_seconds`, `as_unix_seconds`, `to_utc`, `to_utc_with`                                                            | 🟡 Stabil bis 1.0    | Unix-Interop-Methoden, in `#TIME-21` hinzugefügt.                                                                                                                                                                                            |
| `Time::<Utc>::from_unix_seconds`, `from_unix_nanos`, `as_unix_seconds`, `as_unix_nanos`, `to_gps`, `to_gps_with`, `to_civil`            | 🟡 Stabil bis 1.0    | Ebenso.                                                                                                                                                                                                                                       |

### `duration` (von Schicht 2 genutzt, im Übrigen unabhängig)

| Element                                                                                                                                  | Stabilität                       | Anmerkungen                                                                                                                                                                                                                                                                         |
| ---------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Duration`                                                                                                                               | 🟡 Stabil bis 1.0              | Vorzeichenbehafteter Intervall-Typ ([I-5](INVARIANTS.de.md#i-5-duration-ist-vorzeichenbehaftet)).                                                                                                                                                                                    |
| `ZERO`, `MIN`, `MAX`, `ONE_NANOSECOND`                                                                                                   | 🟢 Stabil                        | Assoziierte Konstanten.                                                                                                                                                                                                                                                             |
| `from_nanos`, `from_seconds`, `from_millis`                                                                                              | 🟢 Stabil                        | Kern-Konstruktoren.                                                                                                                                                                                                                                                                 |
| `from_minutes`, `from_hours`, `from_days`                                                                                                | 🟠 Instabil — **deprecated**     | In `#TIME-27.1` wegen der Gefahr eines stillen Überlaufs markiert; `#[deprecated]` zugunsten von `checked_from_*`. Bleiben funktionsfähig (patch-sicher laut Semver-Tabelle), zur eventuellen Entfernung (**minor**-Bump) vorgesehen, sobald `checked_from_*` einen vollständigen Zyklus durchlaufen haben. |
| `checked_from_seconds`, `checked_from_millis`, `checked_from_micros`, `checked_from_minutes`, `checked_from_hours`, `checked_from_days` | 🟡 Stabil bis 1.0              | In `#TIME-27.1` hinzugefügt; die vorgesehene langfristige Ablösung der als veraltet markierten `from_*`-Familie oben.                                                                                                                                                                             |
| `checked_add`, `abs`, `neg` (`Neg`), `is_negative`, `as_seconds`, Arithmetik-Operatoren                                                  | 🟢 Stabil                        |                                                                                                                                                                                                                                                                                     |

### `leap` (Schicht 3)

| Element                                       | Stabilität             | Anmerkungen                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LeapSecondsProvider` (Trait)                 | 🟡 Stabil bis 1.0    | Öffentlich, **nicht** versiegelt — anders als `TimeScale` sind Drittanbieter-Provider (z.B. ein auf Empfänger-Firmware gestützter) ein beabsichtigter Erweiterungspunkt. Dass `tai_minus_utc_at` einen `i32` zurückgibt (löst niemals einen Panic aus, niemals `None` — eine leere Tabelle fällt auf den eingebauten `19 s`-Offset zurück), ist selbst eine [Invariante](INVARIANTS.de.md#i-16-wann-kann-gps--utc-ambiguousleapsecond-erzeugen), fixiert durch den Empty-Table-Audit-Fix von `#TIME-27.1`. |
| `LeapSeconds`                                | 🟡 Stabil bis 1.0    | `builtin()`, `from_table`, `try_from_slice`, `entries`, `last_update`, `current_tai_minus_utc`, `tai_minus_utc_at`.                                                                                                                                                                                                                                                                                                                                                              |
| `RuntimeLeapSeconds`                         | 🟡 Stabil bis 1.0    | `new`, `from_builtin`, `from_slice` (→ `Result<Self, _>`), `try_extend`, `len`, `is_empty`, `entries`, `last_update`, `current_tai_minus_utc`. Der Fehlerprioritäts-Vertrag von `try_extend` ist durch Fuzzing verankert (`fuzz_try_extend.rs`) — als bindend betrachten, nicht als nebensächlich.                                                                                                                                                                                |
| `RUNTIME_CAPACITY`                           | 🟢 Stabil              | `= 64`. Diesen Wert zu ändern ist ein **minor**-Bump (verändert die Größe von `RuntimeLeapSeconds`, eine beobachtbare Eigenschaft nach der [I-18](INVARIANTS.de.md#i-18-8-byte-größe)-nahen Argumentation, auch wenn `RuntimeLeapSeconds` selbst nicht auf 8 Byte festgelegt ist).                                                                                                                                                                                               |
| `LeapEntry`                                  | 🟠 Instabil            | Siehe `#[non_exhaustive]`-Empfehlung oben.                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `LeapExtendError`                            | 🟡 Stabil bis 1.0    | `#[non_exhaustive]`; Variantenmenge durch Fuzzing verankert.                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `gps_to_utc`, `utc_to_gps` (freie Funktionen) | 🟢 Stabil              | Öffentliche Einstiegspunkte des Zwei-Pass-Algorithmus ([I-12](INVARIANTS.de.md#i-12-kontextformel--gps--utc-und-der-zwei-pass-algorithmus-utc--gps)); intensiv gefuzzt.                                                                                                                                                                                                                                                                                                        |

### `convert` (Schicht 4)

| Element                                                       | Stabilität             | Anmerkungen                                                                                                                                                                                                                                             |
| ------------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `IntoScale<Target>` (Trait)                                   | 🟢 Stabil              | Öffentliche API für Konvertierungen mit festem Offset.                                                                                                                                                                                                    |
| `IntoScaleWith<Target>` (Trait)                               | 🟢 Stabil              | Öffentliche API für kontextabhängige Konvertierungen; `into_scale_with` / `into_scale_with_checked`.                                                                                                                                                      |
| `ConvertResult<T>`                                            | 🟡 Stabil bis 1.0    | `is_exact`, `into_inner`, `Exact`/`AmbiguousLeapSecond`; `#[non_exhaustive]`.                                                                                                                                                                            |
| Paarweise `impl IntoScale<…>`/`impl IntoScaleWith<…>`-Blöcke  | 🟢 Stabil              | Nicht einzeln benennbar; ihre *Existenz* für ein gegebenes Paar wird durch die `ConversionMatrix` aus `matrix.rs` dokumentiert — das ist der unterstützte Weg zu erfragen, ob eine Konvertierung existiert, statt Trait-Grenzen zu sondieren.           |

### `matrix` (Schicht 5)

| Element             | Stabilität             | Anmerkungen                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ScaleId`           | 🟠 Instabil            | `#[non_exhaustive]`-enum der sechs Skalen zur Laufzeit-Introspektion; eine neue Skala ergänzen zu wollen ist bereits eine patch-sichere Varianten-Ergänzung, passend zur Patch-Sicherheit des „Neue Skala hinzufügen"-Checklists im Stil von `#TIME-31`/`#TIME-32` (`docs/ARCHITECTURE.de.md#erweiterung-hinzufügen-einer-neuen-zeitskala`).                              |
| `ConversionKind`    | 🟡 Stabil bis 1.0    | `#[non_exhaustive]`; siehe Audit oben.                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `ConversionMatrix`  | 🟠 Instabil            | Laufzeit-Introspektions-API (`path_count`, paarweises `conversion_kind`-Lookup); neueste Schicht, bisher am wenigsten von externen Nutzern verwendet.                                                                                                                                                                                                                                                                                                                                               |

### `civil`

| Element                                                                     | Stabilität             | Anmerkungen                                                                                                                                                                                                                                                                                                                                                                          |
| --------------------------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `CivilDateTime`                                                             | 🟢 Stabil              | Warum es eine einfache Struktur bleibt — siehe `#[non_exhaustive]`-Audit oben; API seit `0.5.3` unverändert (fünf Release-Zyklen).                                                                                                                                                                                                                                                    |
| `from_utc_nanos`, `to_utc_nanos`, `to_utc`, `is_whole_second`, `Display`    | 🟢 Stabil              | Der Algorithmus ist `civil_from_days` von Howard Hinnant (Public Domain); verankert durch Unit-Tests in `src/civil.rs` und Quervergleiche der Kalenderdaten in `tests/time_integration_test.rs`. Kein eigenes Fuzz-Ziel (`fuzz_civil` ist keines der sechs Ziele).                                                                                                                        |
| `Time::<Utc>::to_civil`                                                     | 🟢 Stabil              | Der einzige modulübergreifende Einstiegspunkt; als unfehlbar dokumentiert (siehe den Abschnitt [„Datum und Uhrzeit (ISO 8601)" in `docs/ARCHITECTURE.de.md`](ARCHITECTURE.de.md#datum-und-uhrzeit-iso-8601)).                                                                                                                                                                                         |

### `error`

| Element          | Stabilität | Anmerkungen                                                                                                                                                                                        |
| ---------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GnssTimeError`  | 🟢 Stabil  | `#[non_exhaustive]`; `Overflow`/`InvalidInput`/`LeapSecondsRequired`/`OutOfRange`. `impl std::error::Error` hinter `feature = "std"`; `impl defmt::Format` hinter `feature = "defmt"`.             |

### `serde_impls` (feature = `serde`)

| Element                                                                     | Stabilität                         | Anmerkungen                                                                                                                                                                                                                                              |
| --------------------------------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `impl Serialize`/`Deserialize` für `Time<S>`, `Duration`, `DurationParts`   | 🟡 Stabil bis 1.0                | Das Wire-Format ist in `docs/ARCHITECTURE.de.md#serde-unterstützung-feature--serde` und `docs/EMBEDDED.de.md` dokumentiert; das Format selbst ist der Vertrag, nicht die Elementliste des Moduls.                                                        |
| `pub mod serde_impls` (der Modulpfad selbst)                                | 🔴 Intern — **sollte privat sein** | Siehe [`#[doc(hidden)]`-Empfehlung](#interne-elemente-die-dochidden-erhalten-sollten) unten. Es gibt nichts im Modul, was ein Aufrufer benennen möchte — `impl`-Blöcke werden automatisch durch das Trait-System entdeckt, nicht über den Modulpfad. |

### `prelude`

| Element                                     | Stabilität | Anmerkungen                                                                                                                                                                                                              |
| ------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `gnss_time::prelude::*` (der Glob selbst)   | 🟢 Stabil  | Die Menge der exportierten Namen kann **wachsen** (patch-sicher, rein additiv), aber bestehende Namen werden nicht ohne minor-Bump entfernt. Siehe [Re-Export-Audit](#re-export-audit-für-pub-use) für die aktuelle exakte Liste. |

---

## Re-Export-Audit für `pub use`

`lib.rs` macht derzeit Folgendes:

```rust
pub use civil::CivilDateTime;
pub use convert::*;
pub use duration::*;
pub use epoch::*;
pub use error::*;
pub use leap::*;
pub use matrix::*;
pub use scale::*;
pub use time::*;
```

Jeder Glob wurde gegen die Elemente geprüft, die er tatsächlich an der
Crate-Wurzel re-exportiert (d.h. was als `gnss_time::Foo` statt nur
`gnss_time::module::Foo` erreichbar wird):

| `pub use`             | Was dadurch an der Crate-Wurzel sichtbar wird                                                                                         | Urteil                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `civil::CivilDateTime` | ein benannter Typ                                                                                                                           | ✅ Korrekt — bewusster Einzel-Re-Export, wie bei `Time`/`Duration` an die Oberfläche gebracht.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `convert::*`          | `ConvertResult`, `IntoScale`, `IntoScaleWith`                                                                                               | ✅ Korrekt — der ganze Sinn von Schicht 4 ist es, die primäre aufruferseitige API zu sein.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `duration::*`         | `Duration`, `DurationParts`                                                                                                                 | ✅ Korrekt.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `epoch::*`            | `CivilDate`, alle Epochen-Konstanten (`TAI_EPOCH`, …, `UTC_EPOCH_UNIX_OFFSET_S`, …, `LEAP_SECONDS_AT_*`, `DAYS_GPS_TO_*`, `NANOS_GPS_TO_*`) | ⚠️ **Zu breit.** Dieser Glob bringt rund 20 Elemente an die Crate-Wurzel, von denen die meisten (`LEAP_SECONDS_AT_GLONASS_EPOCH`, `DAYS_GPS_TO_BEIDOU`, `NANOS_GPS_TO_GALILEO_EPOCH`, …) Ableitungskonstanten sind, die intern nur von den eigenen `const`-Assertions in `epoch.rs` und von nichts sonst in der Crate verwendet werden. Sie werden nicht aus `prelude.rs` referenziert, und ihre *Werte* sind bereits vertragsrelevant — jede wird durch die eigenen `const`-Assertions in `epoch.rs` und die durchgerechneten Beispiele in `docs/INVARIANTS.de.md` fixiert —, daher fügt der Re-Export öffentliche Oberfläche hinzu, ohne Flexibilität zu schaffen. Siehe Empfehlung unten.                                                                       |
| `error::*`            | `GnssTimeError`                                                                                                                             | ✅ Korrekt.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `leap::*`             | `LeapSecondsProvider`, `LeapSeconds`, `RuntimeLeapSeconds`, `LeapEntry`, `LeapExtendError`, `RUNTIME_CAPACITY`, `gps_to_utc`, `utc_to_gps` | ✅ Korrekt — alle acht werden aus `prelude.rs` referenziert, was ihren beabsichtigten öffentlichen Status bestätigt.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `matrix::*`           | `ConversionMatrix`, `ScaleId`, `ConversionKind`                                                                                             | ✅ Korrekt — `ScaleId` und `ConversionKind` sind beide `#[non_exhaustive]`, wodurch künftige Skalen-Ergänzungen patch-sicher bleiben.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `scale::*`            | `TimeScale`, `Gps`, `Glonass`, `Galileo`, `Beidou`, `Tai`, `Utc`, `OffsetToTai`, `DisplayStyle`                                             | ⚠️ **`DisplayStyle` markiert.** Der *Typ* muss öffentlich bleiben — er ist der Typ der öffentlichen assoziierten Konstante `TimeScale::DISPLAY_STYLE` —, aber er existiert ausschließlich, damit `Display for Time<S>` ein Format wählt, und wird von keiner öffentlichen Funktion angenommen oder zurückgegeben; ein Aufrufer hat also keinen Grund, ihn direkt zu *benennen*. `#[doc(hidden)]` (siehe unten) verbirgt ihn, ohne den Vertrag der assoziierten Konstante zu brechen.                                                                                                                                         |
| `time::*`             | `Time`                                                                                                                                      | ✅ Korrekt.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

**Gesamtergebnis:** Zwei überflüssige Re-Exports gefunden — die internen
Ableitungskonstanten von `epoch` und `scale::DisplayStyle`. Keiner ist ein
*Bug* (nichts ist beschädigt), aber beide erweitern die öffentliche API-Oberfläche
um Elemente, auf die man sich nie verlassen sollte — genau die Lücke,
die `#[doc(hidden)]` schließt, ohne das Verhalten zu ändern.

---

## Interne Elemente, die `#[doc(hidden)]` erhalten sollten

`#[doc(hidden)]` lässt ein Element technisch öffentlich (bestehender Code, der
zufällig darauf verweist, bricht nicht), entfernt es aber aus generierten
Dokumentationen und signalisiert jedem, der die API durchsucht, „nicht
verwenden". Es ist hier das richtige Werkzeug, weil keins der untenstehenden
Elemente *tatsächlich* privat gemacht werden kann, ohne den
Glob-Re-Export-Mechanismus zu brechen, über den sie erreicht werden
(`pub use epoch::*`, `pub use scale::*`) — das Ziel ist, sie nicht mehr zu
bewerben, nicht zu ändern, was kompiliert.

| Element                                                                                                                            | Ort         | Begründung                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `LEAP_SECONDS_AT_GPS_EPOCH`, `LEAP_SECONDS_AT_GLONASS_EPOCH`, `LEAP_SECONDS_AT_GALILEO_EPOCH`, `LEAP_SECONDS_AT_BEIDOU_EPOCH`      | `epoch.rs`  | Nur von den eigenen `const`-Assertions in `epoch.rs` und den durchgerechneten Beispielen in `docs/INVARIANTS.de.md` verwendet. Von `prelude.rs` nicht referenziert.                                                                                                                                                                                                                                                                                                              |
| `DAYS_GPS_TO_GALILEO`, `DAYS_GPS_TO_BEIDOU`, `DAYS_GPS_TO_GLONASS`, `DAYS_UNIX_TO_GPS`                                             | `epoch.rs`  | Ebenso — nur interne Ableitung.                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `NANOS_GPS_TO_GALILEO_EPOCH`, `NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR`                                                                 | `epoch.rs`  | Ebenso.                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `DisplayStyle` (der Typ selbst und seine Varianten)                                                                                 | `scale.rs`  | Der Typ kann nicht privat gemacht werden — er ist der Typ der öffentlichen assoziierten Konstante `TimeScale::DISPLAY_STYLE` —, aber er existiert ausschließlich für das interne `match` in `Display for Time<S>`, und keine öffentliche Funktion nimmt einen `DisplayStyle` an oder gibt ihn zurück; `#[doc(hidden)]` hält `S::DISPLAY_STYLE` nutzbar und signalisiert zugleich „nennen Sie den Typ nicht direkt".                                                         |
| `pub mod serde_impls`                                                                                                               | `lib.rs`    | Das Modul enthält nur Trait-`impl`-Blöcke (automatisch vom Compiler entdeckt, nie über den Pfad referenziert) plus deren private Hilfstypen (`TimeVisitor`, `DurationVisitor`, Enums für Feld-Schlüssel usw. — siehe `docs/ARCHITECTURE.de.md#serde-unterstützung-feature--serde`). Keines dieser Elemente benötigt einen öffentlichen Modulpfad; `#[doc(hidden)] pub mod serde_impls;` hält die `impl`-Blöcke aktiv und entfernt zugleich einen leeren, verwirrenden Eintrag aus der `cargo doc`-Ausgabe. |
| Modul `tables`                                                                                                                      | `lib.rs`    | Bereits `mod tables;` (privat, nicht `pub mod`) — korrekt so; hier nur aufgeführt, um zu bestätigen, dass das Audit es geprüft hat.                                                                                                                                                                                                                                                                                                                                         |

**Was *nicht* versteckt werden sollte**, zur Abgrenzung: `LeapEntry` ist trotz
der Markierung unter `#[non_exhaustive]` oben echte öffentliche API (zum
Aufbau einer eigenen Schaltsekunden-Tabelle ist es erforderlich) und muss
vollständig dokumentiert bleiben.

---

## Abdeckung der Doc-Tests

Jeder `rust`-Codeblock in einem Doc-Kommentar eines öffentlichen Elements wurde
darauf geprüft, ob er als normaler Doc-Test kompiliert (d.h. ob er nicht als
`ignore` markiert ist, `no_run` dort, wo `run` tatsächlich funktionieren würde,
oder `compile_fail` dort, wo die Absicht ausführbarer Code ist).

**Audit-Ergebnis:** Jeder `rust`-Codeblock in den derzeitigen Doc-Kommentaren
des Crates ist ein schlichter, nicht annotierter Fence — das heißt,
`cargo test --doc` kompiliert und führt sie bereits alle aus, ohne irgendwo im
Crate `ignore`-Schlupflöcher. Das umfasst die Beispiele auf Modulebene in
`lib.rs`, `prelude.rs`, `civil.rs` und alle methodenbezogenen Beispiele in
`time.rs`, `duration.rs`, `leap.rs`, `convert.rs` und `epoch.rs`.

Die eine legitime Verwendung einer nicht standardmäßigen Annotation ist
`examples/no_domain_mixing.rs` — eine *eigenständige Beispieldatei*, die einen
**Kompilierfehler** demonstriert (Mischung von `Time<Gps>`- und
`Time<Glonass>`-Arithmetik) — das ist kein Doc-Test und wird von
`cargo test --doc` überhaupt nicht gebaut; sie existiert rein als
dokumentierte, menschenlesbare Illustration, auf die in
`docs/INVARIANTS.de.md` ([I-1](INVARIANTS.de.md#i-1-domänenisolierung))
verwiesen wird, und ist aus genau diesem Grund korrekt von der
`[[example]]`-Buildliste des Crates ausgeschlossen (sonst würde
`cargo build --examples` fehlschlagen — und genau das ist beabsichtigt).

**Keine Maßnahme nötig** über die laufende Durchsetzung hinaus — siehe unten.

---

## Durchsetzung in CI

`cargo test --doc` war bisher kein eigens benannter CI-Schritt (es läuft
implizit als Teil von `cargo test`, aber ein Fehlschlag dort ist leicht unter
hunderten Namen von Unit- und Integrationstests zu übersehen). Fügen Sie einen expliziten
Schritt hinzu, damit ein kaputter Doc-Test im CI-Output eindeutig erkennbar
ist:

```yaml
# .github/workflows/ci.yml — neben dem bestehenden Test-Job hinzufügen
  doctest:
    name: doc-tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: cargo test --doc (default features)
        run: cargo test --doc
      - name: cargo test --doc (all features)
        run: cargo test --doc --all-features
```

Einen Lauf mit Standard-Features und einen mit `--all-features` auszuführen,
ist genau wegen der `# #[cfg(feature = "serde")] { … }`-Doc-Beispiele in
`lib.rs` und der serde-gesteuerten Fragmente in `civil.rs` wichtig — ein Doc-Test
innerhalb eines `#[cfg(feature = "...")]`-Blocks wird stillschweigend
übersprungen (nicht als Fehlschlag markiert), wenn das Feature aus ist, daher
ist der `--all-features`-Lauf der einzige, der diese Beispiele tatsächlich
kompiliert und ausführt.

`justfile` erhält ein passendes Rezept für die lokale Nutzung:

```just
# Alle Doc-Tests ausführen (default + all-features)
doctest:
    cargo test --doc
    cargo test --doc --all-features
```

und das bestehende `ci`-Rezept (`lint test-all check-embedded doc-check`) sollte
es aufrufen: `ci: lint test-all doctest check-embedded doc-check`.
