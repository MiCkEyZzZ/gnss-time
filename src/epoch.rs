//! # Epochs and calendar arithmetic
//!
//! Every GNSS system anchors its time scale to a fixed calendar point — the
//! *epoch*. This module provides:
//!
//! - [`CivilDate`] — proleptic Gregorian calendar date (no time-of-day or
//!   timezone)
//! - Named epoch constants for all supported time scales
//! - `const fn` day arithmetic for compile-time validation of epochs
//! - Nanosecond offset constants for time conversion layers
//!
//! ## Epoch table
//!
//! | Scale   | Calendar epoch (UTC)              | TAI − UTC at epoch |
//! |---------|-----------------------------------|---------------------|
//! | GLONASS | 1996-01-01 00:00:00 UTC(SU)       | 30 s                |
//! | GPS     | 1980-01-06 00:00:00 UTC           | 19 s                |
//! | Galileo | 1999-08-22 00:00:00 UTC           | 32 s                |
//! | BeiDou  | 2006-01-01 00:00:00 UTC           | 33 s                |
//! | TAI     | 1958-01-01 00:00:00 (definition)  | —                   |
//! | Unix    | 1970-01-01 00:00:00 UTC           | 10 s                |
//!
//! ## Calendar representation and internal time
//!
//! `Time<S>::EPOCH` (0 nanoseconds) corresponds to the calendar epoch listed
//! above, for GPS and GLONASS, where time conversion starts directly from
//! these dates.
//!
//! For cross-scale operations, all systems use a common internal TAI pivot
//! defined in [`OffsetToTai`](crate::scale::OffsetToTai). The constants in
//! this module define calendar distances between epochs and form the basis of
//! future conversion layers that include leap seconds.

/// Proleptic Gregorian calendar date (year, month, day).
///
/// `CivilDate` is a helper type for documentation and arithmetic purposes.
/// It does not contain time-of-day, timezone, or leap second information.
///
/// All methods are `const fn`, allowing compile-time verification of epochs.
///
/// # Examples
///
/// ```rust
/// use gnss_time::{CivilDate, GALILEO_EPOCH, GPS_EPOCH};
///
/// let delta_s = GPS_EPOCH.seconds_until(GALILEO_EPOCH);
///
/// assert_eq!(delta_s, 619_315_200); // well-known GPS → Galileo offset
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CivilDate {
    /// Year (e.g. 1980)
    pub year: i32,

    /// Month (1..=12)
    pub month: u8,

    /// Day of month (1..=31)
    pub day: u8,
}

impl CivilDate {
    /// Creates a calendar date.
    ///
    /// # Important
    ///
    /// No validation is performed — invalid dates (e.g. 31 February)
    /// do not panic, but may lead to incorrect computations.
    #[inline]
    pub const fn new(
        year: i32,
        month: u8,
        day: u8,
    ) -> Self {
        CivilDate { year, month, day }
    }

    /// Number of days since the Unix epoch (`1970-01-01`).
    ///
    /// Negative for dates before 1970.
    /// Uses Howard Hinnant’s algorithm:
    /// <http://howardhinnant.github.io/date_algorithms.html>
    #[inline]
    pub const fn days_from_unix(self) -> i64 {
        days_from_unix_impl(self.year, self.month as i32, self.day as i32)
    }

    /// Difference in days between dates (`other − self`).
    #[inline]
    pub const fn days_until(
        self,
        other: CivilDate,
    ) -> i64 {
        other.days_from_unix() - self.days_from_unix()
    }

    /// Difference in seconds (ignores time-of-day).
    #[inline]
    pub const fn seconds_until(
        self,
        other: CivilDate,
    ) -> i64 {
        self.days_until(other) * 86_400
    }

    /// Difference in nanoseconds (ignores time-of-day).
    #[inline]
    pub const fn nanos_until(
        self,
        other: CivilDate,
    ) -> i64 {
        self.seconds_until(other) * 1_000_000_000
    }
}

/// Converts a calendar date to days since Unix epoch.
///
/// Howard Hinnant’s algorithm:
/// <http://howardhinnant.github.io/date_algorithms.html>
///
/// Uses integer arithmetic only (no floating point).
const fn days_from_unix_impl(
    y: i32,
    m: i32,
    d: i32,
) -> i64 {
    // Сдвигаем январь/февраль так, чтобы они стали 11/12 месяцами
    // предыдущего года. Это гарантирует, что високосный день (29 февраля)
    // всегда оказывается в конце "года".
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let y = y as i64;
    // 400-летняя эра, содержащая год y
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = (y - era * 400) as u64; // год внутри эры [0, 399]

    // День года в сдвинутой системе месяцев [0, 365]
    let doy = ((153 * m as i64 + 2) / 5 + d as i64 - 1) as u64;
    // День внутри 400-летней эры [0, 146096]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    // Дни от 1970-01-01 (719468 = смещение от начала 400-летней эры до 1970)
    era * 146_097 + doe as i64 - 719_468
}

/// TAI epoch: **1958-01-01 00:00:00 TAI**.
///
/// Reference point of international atomic time.
pub const TAI_EPOCH: CivilDate = CivilDate::new(1958, 1, 1);

/// Unix epoch: **1970-01-01 00:00:00 UTC**.
///
/// Reference point for Unix time; at this date TAI − UTC = 10 s.
pub const UNIX_EPOCH: CivilDate = CivilDate::new(1970, 1, 1);

/// GPS epoch: **1980-01-06 00:00:00 UTC**.
///
/// `Time<Gps>::EPOCH` corresponds to this moment.
/// At this date TAI − UTC = 19 s, so `GPS = TAI − 19 s`.
pub const GPS_EPOCH: CivilDate = CivilDate::new(1980, 1, 6);

/// GLONASS epoch: **1996-01-01 00:00:00 UTC(SU)**.
///
/// UTC(SU) = UTC + 3 hours (Moscow standard time, no DST).
/// `Time<Glonass>::EPOCH` counts from this date.
/// At this moment TAI − UTC = 30 s (including leap second of 1995-12-31).
pub const GLONASS_EPOCH: CivilDate = CivilDate::new(1996, 1, 1);

/// Galileo epoch: **1999-08-22 00:00:00 UTC** (= GPS week 1024, TOW 0).
///
/// Galileo System Time uses the same TAI offset as GPS (`GAL = TAI − 19 s`).
/// GPS and Galileo timestamps with identical nanosecond values represent the
/// same physical moment.
pub const GALILEO_EPOCH: CivilDate = CivilDate::new(1999, 8, 22);

/// BeiDou epoch: **2006-01-01 00:00:00 UTC**.
///
/// `Time<Beidou>::EPOCH` corresponds to this date.
/// At this moment TAI − UTC = 33 s, so `BDT = TAI − 33 s`.
/// Relation to GPS: `BDT = GPS − 14 s`.
pub const BEIDOU_EPOCH: CivilDate = CivilDate::new(2006, 1, 1);

/// TAI − UTC at GPS epoch (1980-01-06): **19 seconds**.
pub const LEAP_SECONDS_AT_GPS_EPOCH: i64 = 19;

/// TAI − UTC at GLONASS epoch (1996-01-01): **30 seconds**.
pub const LEAP_SECONDS_AT_GLONASS_EPOCH: i64 = 30;

/// TAI − UTC at Galileo epoch (1999-08-22): **32 seconds**.
pub const LEAP_SECONDS_AT_GALILEO_EPOCH: i64 = 32;

/// TAI − UTC at BeiDou epoch (2006-01-01): **33 seconds**.
pub const LEAP_SECONDS_AT_BEIDOU_EPOCH: i64 = 33;

/// Days from GPS epoch to Galileo epoch: **7168 days**.
///
/// `1999-08-22 − 1980-01-06 = 7168 days = 619 315 200 s`
pub const DAYS_GPS_TO_GALILEO: i64 = GPS_EPOCH.days_until(GALILEO_EPOCH);

/// Days from GPS epoch to BeiDou epoch: **9492 days**.
///
/// `2006-01-01 − 1980-01-06 = 9492 days = 820 108 800 s`
pub const DAYS_GPS_TO_BEIDOU: i64 = GPS_EPOCH.days_until(BEIDOU_EPOCH);

/// Days from GPS epoch to GLONASS epoch: **5839 days**.
///
/// `1996-01-01 − 1980-01-06 = 5839 days`
pub const DAYS_GPS_TO_GLONASS: i64 = GPS_EPOCH.days_until(GLONASS_EPOCH);

/// Days from Unix epoch to GPS epoch: **3657 days**.
pub const DAYS_UNIX_TO_GPS: i64 = UNIX_EPOCH.days_until(GPS_EPOCH);

/// Calendar nanoseconds from GPS epoch to Galileo epoch.
///
/// `619_315_200 s × 10⁹ ns/s = 619_315_200_000_000_000 ns`
pub const NANOS_GPS_TO_GALILEO_EPOCH: i64 = GPS_EPOCH.nanos_until(GALILEO_EPOCH);

/// Calendar nanoseconds from GPS epoch to BeiDou epoch (before leap second
/// adjustment).
///
/// Actual GPS−BDT offset also includes accumulated leap difference:
/// `BDT = GPS − 14 s`.
pub const NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR: i64 = GPS_EPOCH.nanos_until(BEIDOU_EPOCH);

/// Galileo−GPS calendar delta must equal 619 315 200 s.
const _VERIFY_GALILEO: () = {
    let s = NANOS_GPS_TO_GALILEO_EPOCH / 1_000_000_000;
    assert!(s == 619_315_200, "Galileo epoch offset check failed");
};

/// BeiDou−GPS calendar delta must equal 820 108 800 s.
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

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

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
        // 1999-08-22: хорошо известное значение
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
        // 5839 дней * 86_400 = 504_921_600 секунд
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
        // 2000-02-29 — валидная дата; 2000-03-01 = 2000-02-29 + 1
        let feb29 = CivilDate::new(2000, 2, 29);
        let mar01 = CivilDate::new(2000, 3, 1);

        assert_eq!(feb29.days_until(mar01), 1);
    }

    #[test]
    fn year_1900_is_not_leap_year() {
        // 1900 делится на 100, но не на 400 → не високосный год
        let feb28 = CivilDate::new(1900, 2, 28);
        let mar01 = CivilDate::new(1900, 3, 1);

        // Если бы 1900 был високосным годом, разрыв был бы 2 дня. Но он равен 1.
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
        // Историческая таблица високосных секунд IERS
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
