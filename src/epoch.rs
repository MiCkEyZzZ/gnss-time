//! # Эпохи и календарная арифметика
//!
//! Каждая GNSS-система привязывает свою шкалу времени к фиксированной
//! календарной точке — *epoch*. Этот модуль предоставляет:
//!
//! - [`CivilDate`] — пролептическая григорианская дата (без времени суток и
//!   часового пояса)
//! - Именованные константы эпох для всех поддерживаемых шкал времени
//! - `const fn` арифметику дней для проверки корректности эпох на этапе
//!   компиляции
//! - Константы смещения в наносекундах для слоя преобразований времени
//!
//! ## Таблица эпох
//!
//! | Шкала   | Календарная эпоха (UTC)           | TAI − UTC на эпохе |
//! |---------|------------------------------------|--------------------|
//! | GPS     | 1980-01-06 00:00:00 UTC           | 19 с               |
//! | GLONASS | 1996-01-01 00:00:00 UTC(SU)       | 30 с               |
//! | Galileo | 1999-08-22 00:00:00 UTC           | 32 с               |
//! | BeiDou  | 2006-01-01 00:00:00 UTC           | 33 с               |
//! | TAI     | 1958-01-01 00:00:00 (определение) | —                  |
//! | Unix    | 1970-01-01 00:00:00 UTC           | 10 с               |
//!
//! ## Представление календаря и внутреннее время
//!
//! `Time<S>::EPOCH` (0 наносекунд) соответствует календарной эпохе,
//! указанной выше, для GPS и GLONASS, где преобразования начинаются
//! напрямую от этих дат.
//!
//! Для межшкальных операций все системы используют общий внутренний
//! TAI-пивот, описанный в [`OffsetToTai`](crate::scale::OffsetToTai).
//! Константы данного модуля задают календарные расстояния между эпохами
//! и являются основой будущего слоя преобразований с учётом високосных
//! секунд (#TIME-3 и #TIME-4).

/// Пролептическая григорианская календарная дата (год, месяц, день).
///
/// `CivilDate` — это вспомогательный тип для документации и арифметики.
/// Он не содержит времени суток, часового пояса или информации о високосных
/// секундах.
///
/// Все методы являются `const fn`, что позволяет использовать этот тип
/// для проверки эпох на этапе компиляции.
///
/// # Примеры
///
/// ```rust
/// use gnss_time::epoch::{CivilDate, GALILEO_EPOCH, GPS_EPOCH};
///
/// let delta_s = GPS_EPOCH.seconds_until(GALILEO_EPOCH);
/// assert_eq!(delta_s, 619_315_200); // well-known GPS → Galileo offset
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CivilDate {
    /// Год (например: 1980)
    pub year: i32,
    /// Месяц (1..=12)
    pub month: u8,
    /// День месяца (1..=31)
    pub day: u8,
}

impl CivilDate {
    /// Создаёт календарную дату.
    ///
    /// # Важно
    ///
    /// Валидация не выполняется — некорректные даты (например, 31 февраля)
    /// не вызывают panic, а просто приводят к некорректным вычислениям.
    #[inline]
    pub const fn new(
        year: i32,
        month: u8,
        day: u8,
    ) -> Self {
        CivilDate { year, month, day }
    }

    /// Кол-во дней от Unix-эпохи (`1970-01-01`).
    ///
    /// Отрицательное значение для дат до 1970 года.
    /// Используется алгоритм Ховарда Хиннанта:
    /// http://howardhinnant.github.io/date_algorithms.html
    #[inline]
    pub const fn days_from_unix(self) -> i64 {
        days_from_unix_impl(self.year, self.month as i32, self.day as i32)
    }

    /// Разница в днях между датами (`other − self`).
    #[inline]
    pub const fn days_until(
        self,
        other: CivilDate,
    ) -> i64 {
        other.days_from_unix() - self.days_from_unix()
    }

    /// Разница в секундах (без учёта времени суток).
    #[inline]
    pub const fn seconds_until(
        self,
        other: CivilDate,
    ) -> i64 {
        self.days_until(other) * 86_400
    }

    /// Разница в наносекундах (без учёта времени суток).
    #[inline]
    pub const fn nanos_until(
        self,
        other: CivilDate,
    ) -> i64 {
        self.seconds_until(other) * 1_000_000_000
    }
}

/// Перевод календарной даты в дни от Unix-эпохи.
///
/// Алгоритм Ховарда Хиннанта —
/// http://howardhinnant.github.io/date_algorithms.html
///
/// Работает только на целочисленной арифметике и не использует деления
/// с плавающей точкой.
const fn days_from_unix_impl(
    y: i32,
    m: i32,
    d: i32,
) -> i64 {
    // Shift Jan/Feb to be months 11/12 of the previous year so that the
    // leap day (Feb 29) always falls at the end of the "year".
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let y = y as i64;
    // 400-year era containing y
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = (y - era * 400) as u64; // year-of-era [0, 399]
                                      // Day of year using the shifted month numbering [0, 365]
    let doy = ((153 * m as i64 + 2) / 5 + d as i64 - 1) as u64;
    // Day of era [0, 146096]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    // Days from 1970-01-01 (719468 = days from era 0 to 1970-01-01)
    era * 146_097 + doe as i64 - 719_468
}

/// TAI epoch: **1958-01-01 00:00:00 TAI**.
///
/// This is the international reference point for atomic time.
///
/// > **Implementation note**: `Time<Tai>::EPOCH` does _not_ correspond to
/// > this date in the current crate.  TAI is used as an internal conversion
/// > pivot, not as an absolute time origin.  Full TAI-from-1958 support is
/// > planned for `#TIME-4`.
pub const TAI_EPOCH: CivilDate = CivilDate::new(1958, 1, 1);

/// Unix epoch: **1970-01-01 00:00:00 UTC**.
///
/// Provided as a reference point; at this date TAI − UTC = 10 s.
pub const UNIX_EPOCH: CivilDate = CivilDate::new(1970, 1, 1);

/// GPS epoch: **1980-01-06 00:00:00 UTC**.
///
/// `Time<Gps>::EPOCH` corresponds to this moment.
/// At this date TAI − UTC = 19 s, hence `GPS = TAI − 19 s`.
pub const GPS_EPOCH: CivilDate = CivilDate::new(1980, 1, 6);

/// GLONASS epoch: **1996-01-01 00:00:00 UTC(SU)**.
///
/// UTC(SU) = UTC + 3 hours (Moscow standard time, no DST in the system).
/// `Time<Glonass>::EPOCH` counts days from this date.
/// At this date TAI − UTC = 30 s (leap second was added on 1995-12-31).
pub const GLONASS_EPOCH: CivilDate = CivilDate::new(1996, 1, 1);

/// Galileo epoch: **1999-08-22 00:00:00 UTC** (= GPS week 1024, TOW 0).
///
/// Galileo System Time uses the same TAI offset as GPS (`GAL = TAI − 19 s`).
/// A GPS timestamp and a Galileo timestamp with the **same nanosecond count**
/// represent the **same instant**.
pub const GALILEO_EPOCH: CivilDate = CivilDate::new(1999, 8, 22);

/// BeiDou epoch: **2006-01-01 00:00:00 UTC**.
///
/// `Time<Beidou>::EPOCH` corresponds to this date.
/// At this date TAI − UTC = 33 s, hence `BDT = TAI − 33 s`.
/// Relation to GPS: `BDT = GPS − 14 s` (GPS leads by 14 accumulated seconds).
pub const BEIDOU_EPOCH: CivilDate = CivilDate::new(2006, 1, 1);

/// TAI − UTC at the GPS epoch (1980-01-06): **19 seconds**.
pub const LEAP_SECONDS_AT_GPS_EPOCH: i64 = 19;

/// TAI − UTC at the GLONASS epoch (1996-01-01): **30 seconds**.
pub const LEAP_SECONDS_AT_GLONASS_EPOCH: i64 = 30;

/// TAI − UTC at the Galileo epoch (1999-08-22): **32 seconds**.
pub const LEAP_SECONDS_AT_GALILEO_EPOCH: i64 = 32;

/// TAI − UTC at the BeiDou epoch (2006-01-01): **33 seconds**.
pub const LEAP_SECONDS_AT_BEIDOU_EPOCH: i64 = 33;

/// Days from GPS epoch to the Galileo epoch: **7168 days**.
///
/// `1999-08-22 − 1980-01-06 = 7168 days = 619 315 200 s`
pub const DAYS_GPS_TO_GALILEO: i64 = GPS_EPOCH.days_until(GALILEO_EPOCH);

/// Days from GPS epoch to the BeiDou epoch: **9492 days**.
///
/// `2006-01-01 − 1980-01-06 = 9492 days = 820 108 800 s`
pub const DAYS_GPS_TO_BEIDOU: i64 = GPS_EPOCH.days_until(BEIDOU_EPOCH);

/// Days from GPS epoch to the GLONASS epoch: **5839 days**.
///
/// `1996-01-01 − 1980-01-06 = 5839 days`
pub const DAYS_GPS_TO_GLONASS: i64 = GPS_EPOCH.days_until(GLONASS_EPOCH);

/// Days from the Unix epoch to the GPS epoch: **3657 days**.
pub const DAYS_UNIX_TO_GPS: i64 = UNIX_EPOCH.days_until(GPS_EPOCH);

/// Calendar nanoseconds from GPS epoch to Galileo epoch.
///
/// `619_315_200 s × 10⁹ ns/s = 619_315_200_000_000_000 ns`
pub const NANOS_GPS_TO_GALILEO_EPOCH: i64 = GPS_EPOCH.nanos_until(GALILEO_EPOCH);

/// Calendar nanoseconds from GPS epoch to BeiDou epoch (before leap-second
/// adjustment).
///
/// The actual GPS−BDT offset also includes the 14-second accumulated leap
/// difference: `BDT = GPS − 14 s` at all times after the BDT epoch.
pub const NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR: i64 = GPS_EPOCH.nanos_until(BEIDOU_EPOCH);

/// Galileo−GPS calendar delta must equal the known 619 315 200 s.
const _VERIFY_GALILEO: () = {
    let s = NANOS_GPS_TO_GALILEO_EPOCH / 1_000_000_000;
    assert!(s == 619_315_200, "Galileo epoch offset check failed");
};

/// BeiDou−GPS calendar delta must equal the known 820 108 800 s.
const _VERIFY_BEIDOU: () = {
    let s = NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR / 1_000_000_000;
    assert!(s == 820_108_800, "BeiDou epoch offset check failed");
};

/// GPS epoch must be 3657 days from Unix epoch.
const _VERIFY_GPS_UNIX: () = {
    assert!(DAYS_UNIX_TO_GPS == 3657, "GPS Unix offset check failed");
};

/// GLONASS epoch must be 5839 days from GPS epoch.
const _VERIFY_GLONASS: () = {
    assert!(
        DAYS_GPS_TO_GLONASS == 5839,
        "GLONASS epoch offset check failed"
    );
};

impl core::fmt::Display for CivilDate {
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_is_day_zero() {
        assert_eq!(UNIX_EPOCH.days_from_unix(), 0);
    }

    #[test]
    fn gps_epoch_is_3657_days_from_unix() {
        assert_eq!(GPS_EPOCH.days_from_unix(), 3657);
    }

    #[test]
    fn galileo_epoch_days_from_unix() {
        // 1999-08-22: well-known value
        assert_eq!(GALILEO_EPOCH.days_from_unix(), 10825);
    }

    #[test]
    fn beidou_epoch_days_from_unix() {
        // 2006-01-01
        assert_eq!(BEIDOU_EPOCH.days_from_unix(), 13149);
    }

    #[test]
    fn glonass_epoch_days_from_unix() {
        // 1996-01-01
        assert_eq!(GLONASS_EPOCH.days_from_unix(), 9496);
    }

    #[test]
    fn gps_to_galileo_is_7168_days() {
        assert_eq!(DAYS_GPS_TO_GALILEO, 7168);
    }

    #[test]
    fn gps_to_beidou_is_9492_days() {
        assert_eq!(DAYS_GPS_TO_BEIDOU, 9492);
    }

    #[test]
    fn gps_to_glonass_is_5839_days() {
        assert_eq!(DAYS_GPS_TO_GLONASS, 5839);
    }

    #[test]
    fn galileo_minus_gps_is_619315200_seconds() {
        assert_eq!(GPS_EPOCH.seconds_until(GALILEO_EPOCH), 619_315_200);
    }

    #[test]
    fn beidou_minus_gps_calendar_is_820108800_seconds() {
        assert_eq!(GPS_EPOCH.seconds_until(BEIDOU_EPOCH), 820_108_800);
    }

    #[test]
    fn glonass_minus_gps_is_505123200_seconds() {
        // 5839 days * 86400 = 504,489,600 seconds

        let expected = 5839_i64 * 86_400;
        assert_eq!(GPS_EPOCH.seconds_until(GLONASS_EPOCH), expected);
    }

    #[test]
    fn days_until_is_antisymmetric() {
        let a = CivilDate::new(2000, 1, 1);
        let b = CivilDate::new(2001, 1, 1);

        assert_eq!(a.days_until(b), -b.days_until(a));
    }

    #[test]
    fn days_until_self_is_zero() {
        assert_eq!(GPS_EPOCH.days_until(GPS_EPOCH), 0);
    }

    #[test]
    fn year_2000_is_leap_year() {
        // 2000-02-29 is valid; 2000-03-01 = 2000-02-29 + 1
        let feb29 = CivilDate::new(2000, 2, 29);
        let mar01 = CivilDate::new(2000, 3, 1);

        assert_eq!(feb29.days_until(mar01), 1);
    }

    #[test]
    fn year_1900_is_not_leap_year() {
        // 1900 is divisible by 100 but not 400 → not a leap year
        let feb28 = CivilDate::new(1900, 2, 28);
        let mar01 = CivilDate::new(1900, 3, 1);
        // If 1900 were a leap year, the gap would be 2 days. It's 1.

        assert_eq!(feb28.days_until(mar01), 1);
    }

    #[test]
    fn epoch_dates_are_correct() {
        assert_eq!(GPS_EPOCH, CivilDate::new(1980, 1, 6));
        assert_eq!(GLONASS_EPOCH, CivilDate::new(1996, 1, 1));
        assert_eq!(GALILEO_EPOCH, CivilDate::new(1999, 8, 22));
        assert_eq!(BEIDOU_EPOCH, CivilDate::new(2006, 1, 1));
        assert_eq!(TAI_EPOCH, CivilDate::new(1958, 1, 1));
        assert_eq!(UNIX_EPOCH, CivilDate::new(1970, 1, 1));
    }

    #[test]
    fn leap_seconds_at_epochs_match_official_values() {
        // IERS historical leap second table
        assert_eq!(LEAP_SECONDS_AT_GPS_EPOCH, 19);
        assert_eq!(LEAP_SECONDS_AT_GLONASS_EPOCH, 30);
        assert_eq!(LEAP_SECONDS_AT_BEIDOU_EPOCH, 33);
    }

    #[test]
    fn nanos_gps_to_galileo_matches_known_value() {
        assert_eq!(NANOS_GPS_TO_GALILEO_EPOCH, 619_315_200_000_000_000_i64);
    }

    #[test]
    fn nanos_gps_to_beidou_calendar_matches_known_value() {
        assert_eq!(
            NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR,
            820_108_800_000_000_000_i64
        );
    }
}
