//! # Leap seconds — контекст конверсий
//!
//! ## Почему это явный параметр, а не глобал
//!
//! ```text
//! // ❌ Скрытое состояние — плохо
//! let utc = gps.to_utc(); // откуда берутся leap seconds?
//!
//! // ✅ Явный контекст — хорошо
//! let utc = gps_to_utc(gps, LeapSeconds::builtin())?;
//! ```
//!
//! Причины:
//! - `no_std` / embedded: глобальной изменяемой памяти нет
//! - Embedded GNSS-приёмник: таблица читается из алманаха, обновляется в
//!   runtime
//! - Тестирование: легко подставить нужное состояние без mock'ов
//! - Determinism: скомпилированный код не зависит от будущих обновлений IERS
//!
//! ## Поддерживаемые конверсии
//!
//! | Функция             | Контекст leap seconds?     |
//! |---------------------|----------------------------|
//! | `gps_to_utc`        | да                         |
//! | `utc_to_gps`        | да                         |
//! | `glonass_to_utc`    | **нет** (постоянный сдвиг) |
//! | `utc_to_glonass`    | **нет** (постоянный сдвиг) |
//! | `gps_to_glonass`    | да (через UTC)             |
//! | `glonass_to_gps`    | да (через UTC)             |
//!
//! ## ГЛОНАСС и leap seconds
//!
//! ГЛОНАСС отслеживает UTC(SU) = UTC + 3 ч, включая вставку leap seconds.
//! Поэтому конверсия ГЛОНАСС ↔ UTC — это **константный сдвиг** в наносекундах
//! (разница между эпохами), без каких-либо поправок на leap seconds.
//! Leap seconds нужны только при переходе к GPS/Galileo/BeiDou.

use crate::{tables::BUILTIN_TABLE, CivilDate, Glonass, GnssTimeError, Gps, Tai, Time, Utc};

/// Наносекунды от эпохи UTC (1972-01-01) до эпохи ГЛОНАСС (1995-12-31 21:00:00
/// UTC).
///
/// Эпоха ГЛОНАСС = 1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC.
///
/// `UTC_nanos = GLO_nanos + GLONASS_FROM_UTC_EPOCH_NS`
const GLONASS_FROM_UTC_EPOCH_NS: i64 = {
    // от UTC-epoch до 1996-01-01 00:00:00 UTC
    let to_1996 = CivilDate::new(1972, 1, 1).nanos_until(CivilDate::new(1996, 1, 1));

    // минус 3 часа: ГЛОНАСС epoch = 3ч раньше в UTC
    to_1996 - 3 * 3_600 * 1_000_000_000_i64
    // = 8766 дней * 86400 * 1e9 - 10800 * 1e9
    // = 757_382_400_000_000_000 - 10_800_000_000_000 = 757_371_600_000_000_000
};

const _VERIFY_GLONASS_OFFSET: () = {
    let s = GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000;

    assert!(
        s == 757_371_600,
        "GLONASS->UTC epoch offset must be 757371600 s"
    );
};

/// Наносекунды от эпохи UTC (1972-01-01) до эпохи GPS (1980-01-06).
///
/// Эпоха GPS позже, значение положительное.
/// `UTC_nanos_from_1972 = GPS_nanos_from_1980 - (TAI_minus_UTC - 19) * 1e9 +
/// THIS`
const UTC_TO_GPS_EPOCH_NS: i64 = CivilDate::new(1972, 1, 1).nanos_until(CivilDate::new(1980, 1, 6));
// = 2927 дней * 86400 * 1e9 = 252_892_800_000_000_000 ns

const _VERIFY_UTC_GPS_OFFSET: () = {
    let s = UTC_TO_GPS_EPOCH_NS / 1_000_000_000;

    assert!(
        s == 252_892_800,
        "UTC->GPS epoch offset must be 252892800 s (2927 days)"
    );
};

/// Источник поправок TAI-UTC для конверсий со шкалами UTC и ГЛОНАСС.
///
/// Позволяет передавать кастомные таблицы - например, прочитанные из
/// алманаха GNSS-приёмника без изменения кода крейта.
///
/// # Реализация
///
/// ```rust
/// use gnss_time::{LeapEntry, LeapSecondsProvider, Tai, Time};
///
/// struct FixedLeap(i32);
///
/// impl LeapSecondsProvider for FixedLeap {
///     fn tai_minus_utc_at(
///         &self,
///         _tai: Time<Tai>,
///     ) -> i32 {
///         self.0
///     }
/// }
/// ```
pub trait LeapSecondsProvider {
    /// Возвращает TAI - UTC (в секундах) для заданного момента TAI.
    fn tai_minus_utc_at(
        &self,
        tai: Time<Tai>,
    ) -> i32;
}

/// Одна запись в таблице leap seconds.
///
/// Начиная с момента `tai_minus_utc` (внутренние TAI-наносекунды),
/// `TAI - UTC = tai_minus_utc` секунд.
///
/// Контракт (строгий): таблица должна быть отсортирована по `tai_nanos`
/// по возрастанию.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeapEntry {
    /// Внутренние TAI-наносекунды (нижняя граница включительно).
    pub tai_nanos: u64,

    /// TAI - UTC в целых секундах, действующее с этого момента.
    pub tai_minus_utc: i32,
}

/// Статическая таблица поправок leap seconds.
///
/// Встроенная таблица [`builtin`](LeapSeconds::builtin) покрывает все события
/// с момента старта GPS (1980-01-06) по 2017-01-01 включительно.
/// Для времени после последней записи возвращается последнее известное
/// значение (предположение: новых leap seconds нет — стандартная практика).
///
/// # no_std
///
/// `LeapSeconds` хранит `&'static [LeapEntry]` — нет аллокаций, работает везде.
///
/// # Примеры
///
/// ```rust
/// use gnss_time::{
///     leap::{gps_to_utc, LeapSeconds, LeapSecondsProvider},
///     scale::Gps,
///     Time,
/// };
///
/// // Встроенная таблица (до 2017)
/// let ls = LeapSeconds::builtin();
///
/// let gps = Time::<Gps>::from_week_tow(1981, 0.0).unwrap();
/// let utc = gps_to_utc(gps, &ls).unwrap();
/// // GPS ведёт UTC на 18 секунд в этот период
/// ```
pub struct LeapSeconds {
    entries: &'static [LeapEntry], // (Unix секунды, TAI-UTC)
}

impl LeapEntry {
    pub const fn new(
        tai_nanos: u64,
        tai_minus_utc: i32,
    ) -> Self {
        LeapEntry {
            tai_nanos,
            tai_minus_utc,
        }
    }
}

impl LeapSeconds {
    /// Встроенная таблица, действующая по 2017-01-01.
    ///
    /// Охватывает все 18 leap second event эпохи GPS.
    ///
    /// Источник: [IERS Bulletin C](https://www.iers.org/IERS/EN/Publications/Bulletins/bulletins.html)
    pub const fn builtin() -> &'static LeapSeconds {
        static INSTANCE: LeapSeconds = LeapSeconds {
            entries: &BUILTIN_TABLE,
        };

        &INSTANCE
    }

    /// Создаёт из кастомного среза (например, загруженного с приёмника).
    ///
    /// # Требования
    ///
    /// `entries` должен быть отсортирован по `tai_nanos` по возрастанию.
    pub const fn from_table(entries: &'static [LeapEntry]) -> Self {
        Self { entries }
    }

    /// Возвращает кол-во записей в таблице.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` если таблица пуста.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Все записи таблицы (для инспекции / сериализации).
    pub fn entries(&self) -> &[LeapEntry] {
        self.entries
    }
}

impl LeapSecondsProvider for LeapSeconds {
    fn tai_minus_utc_at(
        &self,
        tai: Time<Tai>,
    ) -> i32 {
        let nanos = tai.as_nanos();
        let entries = self.entries;

        if entries.is_empty() {
            return 19; // безопасный fallback значение на GPS-эпоху
        }

        // Находим последнюю запись с tai_nanos <= nanos
        match entries.binary_search_by_key(&nanos, |e| e.tai_nanos) {
            // Точное совпадение: взять найденную запись
            Ok(i) => entries[i].tai_minus_utc,
            // nanos меньше первой записи: вернуть первое значение
            Err(0) => entries[0].tai_minus_utc,
            // Стандартный случай: запись перед точкой вставки
            Err(i) => entries[i - 1].tai_minus_utc,
        }
    }
}

// Blanket impl: &P automatically implements LeapSecondsProvider when P does.
// This allows passing &LeapSeconds::builtin() directly.
impl<P: LeapSecondsProvider> LeapSecondsProvider for &P {
    fn tai_minus_utc_at(
        &self,
        tai: Time<Tai>,
    ) -> i32 {
        (*self).tai_minus_utc_at(tai)
    }
}

/// Конвертация ГЛОНАСС -> UTC (без leap second контекста).
///
/// ГЛОНАСС отслеживает UTC(SU) = UTC + 3ч, включая leap second.
/// Обе шкалы хранят непрерывные наносекунды, поэтому конверсия -
/// это просто прибавление константного смещения между эпохами.
///
/// # Смещение
///
/// `UTC_ns = GLO_ns + 757_371_600_000_000_000`
/// (= дни от UTC_epoch до GLONASS_epoch × 86400 × 1e9)
///
/// # Ошибки
///
///  [`GnssTimeError::Overflow`] — если UTC < UTC-эпохи (1972-01-01).
pub fn glonass_to_utc(glo: Time<Glonass>) -> Result<Time<Utc>, GnssTimeError> {
    let utc_ns = (glo.as_nanos() as i128) + (GLONASS_FROM_UTC_EPOCH_NS as i128);

    if utc_ns < 0 || utc_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }

    Ok(Time::<Utc>::from_nanos(utc_ns as u64))
}

/// Конвертация UTC -> ГЛОНАСС (без leap second контекста).
///
/// # Ошибки
///
/// [`GnssTimeError::Overflow`] — если UTC раньше GLONASS-эпохи (1996-01-01
/// UTC(SU)).
pub fn utc_to_glonass(utc: Time<Utc>) -> Result<Time<Glonass>, GnssTimeError> {
    let glo_ns = (utc.as_nanos() as i128) - (GLONASS_FROM_UTC_EPOCH_NS as i128);

    if glo_ns < 0 || glo_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }

    Ok(Time::<Glonass>::from_nanos(glo_ns as u64))
}

/// Конвертация GLONASS → GPS через UTC.
///
/// Требует leap second контекст (для UTC → GPS).
pub fn glonass_to_gps<P: LeapSecondsProvider>(
    glo: Time<Glonass>,
    ls: &P,
) -> Result<Time<Gps>, GnssTimeError> {
    let utc = glonass_to_utc(glo)?;

    utc_to_gps(utc, ls)
}

/// Конвертация GPS → UTC.
///
/// Требует явного контекста [`LeapSecondsProvider`].
///
/// # Формула
///
/// ```text
/// UTC_nanos_from_1972 = GPS_nanos_from_1980
///                       - (TAI_minus_UTC - 19) * 1e9
///                       + GPS_EPOCH_OFFSET_FROM_UTC_EPOCH_ns
/// ```
///
/// # Ошибки
///
/// [`GnssTimeError::Overflow`] — результат не помещается в `u64`.
///
/// # Примеры
///
/// ```rust
/// use gnss_time::leap::{LeapSeconds, gps_to_utc};
/// use gnss_time::{Time, scale::Gps};
///
/// let ls = LeapSeconds::builtin();
/// let gps = Time::<Gps>::from_nanos(0); // эпоха GPS
/// let utc = gps_to_utc(gps, &ls).unwrap();
/// // На GPS-эпохе (1980-01-06) GPS-UTC = 0; UTC должен показывать ту же точку
/// assert_eq!(utc.as_nanos(), 252_892_800_000_000_000); // с 1972-01-01
/// ```
pub fn gps_to_utc<P: LeapSecondsProvider>(
    gps: Time<Gps>,
    ls: &P,
) -> Result<Time<Utc>, GnssTimeError> {
    let tai = gps.to_tai()?;
    let n = ls.tai_minus_utc_at(tai);
    // UTC_ns = GPS_ns - (n - 19) * 1e9 + epoch_offset
    let utc_ns = (gps.as_nanos() as i128) - ((n - 19) as i128 * 1_000_000_000_i128)
        + (UTC_TO_GPS_EPOCH_NS as i128);

    if utc_ns < 0 || utc_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }

    Ok(Time::<Utc>::from_nanos(utc_ns as u64))
}

/// Конвертация UTC → GPS.
///
/// Требует явного контекста [`LeapSecondsProvider`].
///
/// # Точность при вставке leap second
///
/// В течение 1-секундного окна вставки leap second результат может быть
/// смещён на 1 секунду. Для всех остальных моментов результат точен.
///
/// # Ошибки
///
/// [`GnssTimeError::Overflow`] — результат не помещается в `u64`.
pub fn utc_to_gps<P: LeapSecondsProvider>(
    utc: Time<Utc>,
    ls: &P,
) -> Result<Time<Gps>, GnssTimeError> {
    // Two-pass lookup to handle leap second boundaries correctly.
    //
    // Pass 1: Approximate TAI assuming GPS-UTC = 0. This under-estimates
    // TAI by up to (current GPS-UTC) seconds near a boundary.
    let approx_tai_ns =
        (utc.as_nanos() as i128) - (UTC_TO_GPS_EPOCH_NS as i128) + 19_000_000_000_i128;

    let tai1 = if approx_tai_ns >= 0 && approx_tai_ns <= u64::MAX as i128 {
        Time::<Tai>::from_nanos(approx_tai_ns as u64)
    } else {
        Time::<Tai>::EPOCH
    };

    let n1 = ls.tai_minus_utc_at(tai1);

    // Pass 2: Refine using n1, resolving any boundary ambiguity.
    let refined_tai_ns = (utc.as_nanos() as i128) - (UTC_TO_GPS_EPOCH_NS as i128)
        + (n1 as i128 * 1_000_000_000_i128);

    let tai2 = if refined_tai_ns >= 0 && refined_tai_ns <= u64::MAX as i128 {
        Time::<Tai>::from_nanos(refined_tai_ns as u64)
    } else {
        tai1
    };

    let n = ls.tai_minus_utc_at(tai2);

    let gps_ns = (utc.as_nanos() as i128) + ((n - 19) as i128 * 1_000_000_000_i128)
        - (UTC_TO_GPS_EPOCH_NS as i128);
    if gps_ns < 0 || gps_ns > u64::MAX as i128 {
        return Err(GnssTimeError::Overflow);
    }
    Ok(Time::<Gps>::from_nanos(gps_ns as u64))
}

/// Конвертация GPS → GLONASS через UTC.
///
/// Требует leap second контекст (для GPS → UTC).
pub fn gps_to_glonass<P: LeapSecondsProvider>(
    gps: Time<Gps>,
    ls: &P,
) -> Result<Time<Glonass>, GnssTimeError> {
    let utc = gps_to_utc(gps, ls)?;

    utc_to_glonass(utc)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use std::string::ToString;

    use super::*;
    use crate::scale::Gps;

    #[test]
    fn test_utc_to_gps_epoch_offset_is_252892800_seconds() {
        assert_eq!(UTC_TO_GPS_EPOCH_NS / 1_000_000_000, 252_892_800);
    }

    #[test]
    fn test_utc_to_gps_epoch_offset_is_2927_days() {
        assert_eq!(UTC_TO_GPS_EPOCH_NS / 1_000_000_000 / 86_400, 2927);
    }

    #[test]
    fn test_glonass_epoch_offset_from_utc_epoch_is_correct() {
        // 757_371_600 s = 8766 days * 86400 - 3h
        // = (days from 1972-01-01 to 1996-01-01) * 86400 - 10800
        assert_eq!(GLONASS_FROM_UTC_EPOCH_NS / 1_000_000_000, 757_371_600);
    }

    #[test]
    fn test_builtin_table_is_sorted() {
        let entries = LeapSeconds::builtin().entries();

        for w in entries.windows(2) {
            assert!(
                w[0].tai_nanos < w[1].tai_nanos,
                "table not sorted at {:?}",
                w
            );
        }
    }

    #[test]
    fn test_builtin_table_starts_with_tai_minus_utc_19() {
        assert_eq!(BUILTIN_TABLE[0].tai_minus_utc, 19);
    }

    #[test]
    fn test_builtin_table_ends_with_tai_minus_utc_37() {
        let last = *BUILTIN_TABLE.last().unwrap();

        assert_eq!(last.tai_minus_utc, 37);
    }

    #[test]
    fn test_builtin_table_has_monotone_increasing_tai_minus_utc() {
        let entries = LeapSeconds::builtin().entries();

        for w in entries.windows(2) {
            assert!(
                w[1].tai_minus_utc == w[0].tai_minus_utc + 1,
                "expected each entry to increment by 1"
            );
        }
    }

    #[test]
    fn test_lookup_at_tai_zero_returns_19() {
        let ls = LeapSeconds::builtin();

        assert_eq!(ls.tai_minus_utc_at(Time::<Tai>::EPOCH), 19);
    }

    #[test]
    fn test_lookup_at_max_tai_returns_last_value() {
        let ls = LeapSeconds::builtin();

        assert_eq!(ls.tai_minus_utc_at(Time::<Tai>::MAX), 37);
    }

    #[test]
    fn test_lookup_at_exact_2017_threshold_returns_37() {
        let ls = LeapSeconds::builtin();
        // TAI threshold for 2017-01-01 = 1_167_264_037_000_000_000
        let tai = Time::<Tai>::from_nanos(1_167_264_037_000_000_000);

        assert_eq!(ls.tai_minus_utc_at(tai), 37);
    }

    #[test]
    fn test_lookup_one_ns_before_2017_threshold_returns_36() {
        let ls = LeapSeconds::builtin();
        let tai = Time::<Tai>::from_nanos(1_167_264_037_000_000_000 - 1);

        assert_eq!(ls.tai_minus_utc_at(tai), 36);
    }

    #[test]
    fn test_lookup_at_1999_threshold_returns_32() {
        let ls = LeapSeconds::builtin();
        // TAI threshold for 1999-01-01 = 599_184_032_000_000_000
        let tai = Time::<Tai>::from_nanos(599_184_032_000_000_000);

        assert_eq!(ls.tai_minus_utc_at(tai), 32);
    }

    #[test]
    fn test_lookup_one_ns_before_1999_threshold_returns_31() {
        let ls = LeapSeconds::builtin();
        let tai = Time::<Tai>::from_nanos(599_184_032_000_000_000 - 1);

        assert_eq!(ls.tai_minus_utc_at(tai), 31);
    }

    #[test]
    fn test_gps_utc_gps_roundtrip_at_gps_epoch() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::EPOCH;
        let utc = gps_to_utc(gps, &ls).unwrap();
        let back = utc_to_gps(utc, &ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_utc_gps_roundtrip_at_2020() {
        let ls = LeapSeconds::builtin();
        // GPS 2020-01-01 ≈ week 2086
        let gps = Time::<Gps>::from_week_tow(2086, 0.0).unwrap();
        let utc = gps_to_utc(gps, &ls).unwrap();
        let back = utc_to_gps(utc, &ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_epoch_utc_is_correct_offset_from_utc_epoch() {
        let ls = LeapSeconds::builtin();
        // На GPS-эпохе (1980-01-06) TAI-UTC = 19, GPS-UTC = 0
        // UTC nanos = GPS_nanos - 0 + UTC_TO_GPS_EPOCH_NS
        //           = 0 + 252_892_800_000_000_000
        let utc = gps_to_utc(Time::<Gps>::EPOCH, &ls).unwrap();

        assert_eq!(utc.as_nanos(), 252_892_800_000_000_000);
    }

    /// Проверяем GPS-UTC = 18 на 2017-01-01 00:00:00 UTC.
    ///
    /// GPS на 2017-01-01 (unix=1483228800):
    ///   GPS_s = (1483228800 - 315964800) + (37-19) = 1167264000 + 18 =
    /// 1167264018 UTC nanos от UTC_epoch = 16437 дней * 86400 * 1e9 =
    /// 1_420_156_800_000_000_000
    #[test]
    fn test_gps_minus_utc_is_18s_at_2017_01_01() {
        let ls = LeapSeconds::builtin();
        // GPS секунды для 2017-01-01 00:00:00 UTC
        // = (unix - GPS_EPOCH_UNIX) + (TAI-UTC - 19) = (1483228800 - 315964800) + 18
        let gps_s: u64 = 1_167_264_000 + 18;
        let gps = Time::<Gps>::from_seconds(gps_s);
        let utc = gps_to_utc(gps, &ls).unwrap();

        // UTC nanos for 2017-01-01 = 16437 days * 86400 * 1e9
        let expected_utc_ns: u64 = 16_437 * 86_400 * 1_000_000_000;

        assert_eq!(utc.as_nanos(), expected_utc_ns);
    }

    /// Проверяем GPS-UTC = 13 на 1999-01-01 00:00:00 UTC.
    #[test]
    fn test_gps_minus_utc_is_13s_at_1999_01_01() {
        let ls = LeapSeconds::builtin();
        // GPS_s = (915148800 - 315964800) + (32 - 19) = 599184000 + 13 = 599184013
        let gps = Time::<Gps>::from_seconds(599_184_013);
        let utc = gps_to_utc(gps, &ls).unwrap();

        // UTC от UTC-epoch до 1999-01-01:
        // days_from_unix(1999-01-01) - days_from_unix(1972-01-01)
        // = 10592 - 730 = 9862 дней (проверено ниже)
        // UTC_s = 9862 * 86400 = 851_948_800
        let expected_utc_s: u64 = 9_862 * 86_400;

        assert_eq!(utc.as_seconds(), expected_utc_s);
    }

    /// 1998-12-31 → 1999-01-01: TAI-UTC с 31 → 32, GPS-UTC с 12 → 13.
    ///
    /// GPS перепрыгивает с ...011 на ...013 (нет ...012 в реальном UTC).
    #[test]
    fn test_leap_second_transition_1999_gps_jumps_by_2s() {
        let ls = LeapSeconds::builtin();

        // 1 секунда до перехода: 1998-12-31 23:59:59 UTC
        // unix = 915148799, TAI-UTC = 31 (ещё старое значение)
        // GPS_s = (915148799 - 315964800) + 12 = 599183999 + 12 = 599184011
        let gps_before = Time::<Gps>::from_seconds(599_184_011);

        // Сразу после: 1999-01-01 00:00:00 UTC
        // unix = 915148800, TAI-UTC = 32 (новое значение)
        // GPS_s = (915148800 - 315964800) + 13 = 599184000 + 13 = 599184013
        let gps_after = Time::<Gps>::from_seconds(599_184_013);

        // Оба должны конвертироваться корректно
        let utc_before = gps_to_utc(gps_before, &ls).unwrap();
        let utc_after = gps_to_utc(gps_after, &ls).unwrap();

        // UTC-after - UTC-before = 1 секунда (вставка leap second выровняла счёт)
        let diff = (utc_after - utc_before).as_seconds();

        assert_eq!(diff, 1, "GPS jumped 2s but UTC advanced 1s (leap second)");
    }

    /// 2016-12-31 → 2017-01-01: TAI-UTC 36 → 37, GPS-UTC 17 → 18.
    #[test]
    fn test_leap_second_transition_2017_gps_jumps_by_2s() {
        let ls = LeapSeconds::builtin();

        // 1 секунда до: unix = 1483228799, GPS_s = (1483228799 - 315964800) + 17
        let gps_before = Time::<Gps>::from_seconds(1_167_263_999 + 17);
        // Сразу после: unix = 1483228800, GPS_s = (1483228800 - 315964800) + 18
        let gps_after = Time::<Gps>::from_seconds(1_167_264_000 + 18);

        let utc_before = gps_to_utc(gps_before, &ls).unwrap();
        let utc_after = gps_to_utc(gps_after, &ls).unwrap();

        let diff = (utc_after - utc_before).as_seconds();

        assert_eq!(diff, 1, "GPS jumped 2s but UTC advanced 1s");
    }

    #[test]
    fn test_glonass_epoch_to_utc_gives_correct_nanos() {
        // GLO epoch = 1996-01-01 00:00:00 UTC(SU) = 1995-12-31 21:00:00 UTC
        // UTC от UTC-epoch: (days до 1995-12-31) * 86400 + 21*3600 = ...
        // Мы проверяем через GLONASS_FROM_UTC_EPOCH_NS константу
        let utc = glonass_to_utc(Time::<Glonass>::EPOCH).unwrap();

        assert_eq!(utc.as_nanos(), GLONASS_FROM_UTC_EPOCH_NS as u64);
    }

    #[test]
    fn test_utc_to_glonass_epoch_gives_zero() {
        let utc = Time::<Utc>::from_nanos(GLONASS_FROM_UTC_EPOCH_NS as u64);
        let glo = utc_to_glonass(utc).unwrap();

        assert_eq!(glo, Time::<Glonass>::EPOCH);
    }

    #[test]
    fn test_glonass_utc_glonass_roundtrip() {
        let glo = Time::<Glonass>::from_day_tod(10_000, 43_200.0).unwrap();
        let utc = glonass_to_utc(glo).unwrap();
        let back = utc_to_glonass(utc).unwrap();

        assert_eq!(glo, back);
    }

    #[test]
    fn test_utc_before_glonass_epoch_returns_error() {
        // UTC epoch (1972-01-01) < GLONASS epoch (1996), поэтому UTC=0 → underflow
        let utc = Time::<Utc>::EPOCH;

        assert!(matches!(utc_to_glonass(utc), Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_glonass_offset_is_exactly_3_hours_less_than_day_boundary() {
        // Смещение = 8766 дней * 86400 - 3*3600 (ровно 3 часа до полуночи 1996-01-01
        // UTC)
        let three_hours_ns: i64 = 3 * 3_600 * 1_000_000_000;
        let days_ns: i64 = 8766 * 86_400 * 1_000_000_000;

        assert_eq!(GLONASS_FROM_UTC_EPOCH_NS, days_ns - three_hours_ns);
    }

    #[test]
    fn test_gps_to_glonass_to_gps_roundtrip() {
        let ls = LeapSeconds::builtin();
        // GPS в 2020 году (после последнего leap second 2017)
        let gps = Time::<Gps>::from_week_tow(2100, 86400.0).unwrap();
        let glo = gps_to_glonass(gps, &ls).unwrap();
        let back = glonass_to_gps(glo, &ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_custom_provider_works() {
        struct Always37;

        impl LeapSecondsProvider for Always37 {
            fn tai_minus_utc_at(
                &self,
                _: Time<Tai>,
            ) -> i32 {
                37
            }
        }

        let gps = Time::<Gps>::from_seconds(1_000_000_000);
        let utc = gps_to_utc(gps, &Always37).unwrap();
        let back = utc_to_gps(utc, &Always37).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_empty_table_returns_fallback_19() {
        static EMPTY: [LeapEntry; 0] = [];

        let ls = LeapSeconds::from_table(&EMPTY);

        assert_eq!(
            ls.tai_minus_utc_at(Time::<Tai>::from_seconds(1_000_000)),
            19
        );
    }
}
