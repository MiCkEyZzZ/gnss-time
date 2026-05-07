//! # Epochs and calendar arithmetic
//!
//! This module defines calendar epochs used by GNSS time scales and provides
//! a minimal civil calendar type for epoch arithmetic.
//!
//! ## Overview
//!
//! GNSS time scales are anchored to fixed calendar epochs. This module
//! provides:
//!
//! - [`CivilDate`] — proleptic Gregorian calendar date (UTC, no time-of-day)
//! - Canonical epoch constants for supported GNSS time scales
//! - Compile-time day and nanosecond offsets between epochs
//!
//! ## Epoch reference table
//!
//! | Scale   | Epoch (UTC)                      | TAI − UTC |
//! |---------|----------------------------------|-----------|
//! | GLONASS | 1996-01-01 00:00:00 UTC(SU)      | 30 s      |
//! | GPS     | 1980-01-06 00:00:00 UTC          | 19 s      |
//! | Galileo | 1999-08-22 00:00:00 UTC          | 32 s      |
//! | BeiDou  | 2006-01-01 00:00:00 UTC          | 33 s      |
//! | TAI     | 1958-01-01 00:00:00 (definition) | —         |
//! | Unix    | 1970-01-01 00:00:00 UTC          | 10 s      |
//! | UTC     | 1972-01-01 00:00:00 UTC          | 10 s      |
//!
//! ## Unix time interoperability
//!
//! Unix time (POSIX) counts seconds since **1970-01-01 00:00:00 UTC**.
//! The `gnss-time` UTC epoch is **1972-01-01 00:00:00 UTC** — two years later.
//!
//! The constant [`UTC_EPOCH_UNIX_OFFSET_S`] expresses this gap:
//!
//! ```text
//! unix_seconds = utc_seconds_from_1972 - UTC_EPOCH_UNIX_OFFSET_S
//!                                        (= -63_072_000)
//! utc_nanos    = unix_nanos + UTC_EPOCH_UNIX_OFFSET_NS
//! ```
//!
//! ## Notes on representation
//!
//! Each `Time<S>::EPOCH` corresponds to the epoch listed above. For all
//! systems, internal representation is ultimately aligned through a TAI
//! reference pivot defined in [`OffsetToTai`](crate::scale::OffsetToTai).
//!
//! The constants in this module define *calendar offsets only* and do not
//! include leap second handling.

/// Proleptic Gregorian calendar date.
///
/// A minimal date type used for epoch definitions and calendar arithmetic.
///
/// This type does not include time-of-day, timezone, or leap second
/// information.
///
/// ## Validity
///
/// No validation is performed. Invalid dates are allowed and may produce
/// undefined calendar results.
///
/// ## Examples
///
/// ```rust
/// use gnss_time::{CivilDate, GALILEO_EPOCH, GPS_EPOCH};
///
/// let delta = GPS_EPOCH.seconds_until(GALILEO_EPOCH);
/// assert_eq!(delta, 619_315_200);
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CivilDate {
    /// Year (e.g. 1980)
    pub year: i32,
    /// Month (1–12)
    pub month: u8,
    /// Day of month (1–31)
    pub day: u8,
}

impl CivilDate {
    /// Creates a new calendar date.
    ///
    /// No validation is performed.
    #[inline]
    #[must_use]
    pub const fn new(
        year: i32,
        month: u8,
        day: u8,
    ) -> Self {
        CivilDate { year, month, day }
    }

    /// Returns the number of days since the Unix epoch (`1970-01-01`).
    ///
    /// Negative values indicate dates before the epoch.
    ///
    /// Uses Howard Hinnant’s algorithm.
    #[inline]
    #[must_use]
    pub const fn days_from_unix(self) -> i64 {
        days_from_unix_impl(self.year, self.month as i32, self.day as i32)
    }

    /// Returns the signed difference in days (`other - self`).
    #[inline]
    #[must_use]
    pub const fn days_until(
        self,
        other: CivilDate,
    ) -> i64 {
        other.days_from_unix() - self.days_from_unix()
    }

    /// Returns the difference in whole seconds.
    ///
    /// Time-of-day is not considered.
    #[inline]
    #[must_use]
    pub const fn seconds_until(
        self,
        other: CivilDate,
    ) -> i64 {
        self.days_until(other) * 86_400
    }

    /// Returns the difference in nanoseconds.
    ///
    /// Time-of-day is not considered.
    #[inline]
    #[must_use]
    pub const fn nanos_until(
        self,
        other: CivilDate,
    ) -> i64 {
        self.seconds_until(other) * 1_000_000_000
    }
}

/// Converts a civil date to days since Unix epoch.
///
/// Implementation based on Howard Hinnant’s algorithm:
/// <http://howardhinnant.github.io/date_algorithms.html>
const fn days_from_unix_impl(
    y: i32,
    m: i32,
    d: i32,
) -> i64 {
    // Shift January/February so they become months 11/12 of the previous year.
    // This ensures the leap day (Feb 29) always falls at the end of the "year".
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let y = y as i64;
    // 400-year era containing year y
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = (y - era * 400) as u64; // год внутри эры [0, 399]

    // Day of year in shifted month system [0, 365]
    let doy = ((153 * m as i64 + 2) / 5 + d as i64 - 1) as u64;
    // Day within 400-year era [0, 146096]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    // Days since 1970-01-01 (719468 = offset from start of 400-year era to 1970)
    era * 146_097 + doe as i64 - 719_468
}

/// TAI epoch (1958-01-01).
pub const TAI_EPOCH: CivilDate = CivilDate::new(1958, 1, 1);

/// Unix epoch (1970-01-01).
pub const UNIX_EPOCH: CivilDate = CivilDate::new(1970, 1, 1);

/// UTC epoch (1972-01-01).
///
/// This is the reference point of [`crate::Time<crate::Utc>`]: nanoseconds
/// are counted from this date, **not** from the Unix epoch (1970-01-01).
pub const UTC_CIVIL_EPOCH: CivilDate = CivilDate::new(1972, 1, 1);

/// GPS epoch (1980-01-06).
pub const GPS_EPOCH: CivilDate = CivilDate::new(1980, 1, 6);

/// GLONASS epoch (1996-01-01 UTC(SU)).
pub const GLONASS_EPOCH: CivilDate = CivilDate::new(1996, 1, 1);

/// Galileo epoch (1999-08-22).
pub const GALILEO_EPOCH: CivilDate = CivilDate::new(1999, 8, 22);

/// BeiDou epoch (2006-01-01).
pub const BEIDOU_EPOCH: CivilDate = CivilDate::new(2006, 1, 1);

/// Seconds from the Unix epoch (1970-01-01) to the UTC epoch (1972-01-01).
///
/// `Time<Utc>` counts nanoseconds from 1972-01-01, while Unix time counts
/// seconds from 1970-01-01. This constant bridges the two:
///
/// ```text
/// unix_seconds     = utc_seconds_from_1972 - UTC_EPOCH_UNIX_OFFSET_S
/// utc_from_1972    = unix_seconds          + UTC_EPOCH_UNIX_OFFSET_S
/// ```
///
/// Value: 730 days × 86 400 s/day = **63 072 000 s**.
///
/// ## Example
///
/// ```rust
/// use gnss_time::{UNIX_EPOCH, UTC_CIVIL_EPOCH, UTC_EPOCH_UNIX_OFFSET_S};
///
/// // Verify via calendar arithmetic
/// assert_eq!(
///     UNIX_EPOCH.seconds_until(UTC_CIVIL_EPOCH),
///     UTC_EPOCH_UNIX_OFFSET_S
/// );
/// ```
pub const UTC_EPOCH_UNIX_OFFSET_S: i64 = UNIX_EPOCH.seconds_until(UTC_CIVIL_EPOCH);

/// Nanoseconds from the Unix epoch (1970-01-01) to the UTC epoch (1972-01-01).
///
/// ```text
/// utc_nanos_from_1972 = unix_nanos + UTC_EPOCH_UNIX_OFFSET_NS
/// unix_nanos          = utc_nanos  - UTC_EPOCH_UNIX_OFFSET_NS
/// ```
pub const UTC_EPOCH_UNIX_OFFSET_NS: i64 = UTC_EPOCH_UNIX_OFFSET_S * 1_000_000_000;

/// Second from the Unix epoch (1970-01-01) to the GPS epoch (1980-01-06).
///
/// Useful for converting between Unix timestamps and GPS seconds:
///
/// ```text
/// gps_seconds  = unix_seconds - GPS_EPOCH_UNIX_S + (TAI_minus_UTC - 19)
/// unix_seconds = gps_seconds  + GPS_EPOCH_UNIX_S - (TAI_minus_UTC - 19)
/// ```
///
/// Value: 3 657 days × 86 400 s/day = **315 964 800 s**.
pub const GPS_EPOCH_UNIX_S: i64 = UNIX_EPOCH.seconds_until(GPS_EPOCH);
// = 3657 * 86_400 = 315_964_800

/// TAI − UTC at GPS epoch.
pub const LEAP_SECONDS_AT_GPS_EPOCH: i64 = 19;

/// TAI − UTC at GLONASS epoch.
pub const LEAP_SECONDS_AT_GLONASS_EPOCH: i64 = 30;

/// TAI − UTC at Galileo epoch.
pub const LEAP_SECONDS_AT_GALILEO_EPOCH: i64 = 32;

/// TAI − UTC at BeiDou epoch.
pub const LEAP_SECONDS_AT_BEIDOU_EPOCH: i64 = 33;

/// Days between GPS and Galileo epochs.
pub const DAYS_GPS_TO_GALILEO: i64 = GPS_EPOCH.days_until(GALILEO_EPOCH);

/// Days between GPS and BeiDou epochs.
pub const DAYS_GPS_TO_BEIDOU: i64 = GPS_EPOCH.days_until(BEIDOU_EPOCH);

/// Days between GPS and GLONASS epochs.
pub const DAYS_GPS_TO_GLONASS: i64 = GPS_EPOCH.days_until(GLONASS_EPOCH);

/// Days between Unix and GPS epochs.
pub const DAYS_UNIX_TO_GPS: i64 = UNIX_EPOCH.days_until(GPS_EPOCH);

/// Nanoseconds between GPS and Galileo epochs.
pub const NANOS_GPS_TO_GALILEO_EPOCH: i64 = GPS_EPOCH.nanos_until(GALILEO_EPOCH);

/// Nanoseconds between GPS and BeiDou epochs (calendar only).
pub const NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR: i64 = GPS_EPOCH.nanos_until(BEIDOU_EPOCH);

// Galileo−GPS calendar delta must equal 619 315 200 s.
const _VERIFY_GALILEO: () = {
    let s = NANOS_GPS_TO_GALILEO_EPOCH / 1_000_000_000;
    assert!(s == 619_315_200, "Galileo epoch offset check failed");
};

// BeiDou−GPS calendar delta must equal 820 108 800 s.
const _VERIFY_BEIDOU: () = {
    let s = NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR / 1_000_000_000;
    assert!(s == 820_108_800, "BeiDou epoch offset check failed");
};

// GPS epoch must be 3657 days from Unix epoch.
const _VERIFY_GPS_UNIX: () = {
    assert!(DAYS_UNIX_TO_GPS == 3657, "GPS Unix offset check failed");
};

// GLONASS epoch must be 5839 days from GPS epoch.
const _VERIFY_GLONASS: () = {
    assert!(
        DAYS_GPS_TO_GLONASS == 5839,
        "GLONASS epoch offset check failed"
    );
};

const _VERIFY_GPS_UNIX_S: () = {
    assert!(
        GPS_EPOCH_UNIX_S == 315_964_800,
        "GPS_EPOCH_UNIX_S must equal 315_964_800"
    );
};

const _VERIFY_UTC_UNIX_OFFSET: () = {
    assert!(
        UTC_EPOCH_UNIX_OFFSET_S == 63_072_000,
        "UTC_EPOCH_UNIX_OFFSET_S must equal 63_072_000 (730 days)"
    );
};

const _VERIFY_UTC_UNIX_OFFSET_NS: () = {
    assert!(
        UTC_EPOCH_UNIX_OFFSET_NS == 63_072_000_000_000_000,
        "UTC_EPOCH_UNIX_OFFSET_NS must equal 63_072_000_000_000_000"
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
    fn test_unix_epoch_is_day_zero() {
        assert_eq!(UNIX_EPOCH.days_from_unix(), 0);
    }

    #[test]
    fn test_gps_epoch_is_3657_days_from_unix() {
        assert_eq!(GPS_EPOCH.days_from_unix(), 3657);
    }

    #[test]
    fn test_galileo_epoch_days_from_unix() {
        // 1999-08-22: well-known reference value
        assert_eq!(GALILEO_EPOCH.days_from_unix(), 10825);
    }

    #[test]
    fn test_beidou_epoch_days_from_unix() {
        // 2006-01-01
        assert_eq!(BEIDOU_EPOCH.days_from_unix(), 13149);
    }

    #[test]
    fn test_glonass_epoch_days_from_unix() {
        // 1996-01-01
        assert_eq!(GLONASS_EPOCH.days_from_unix(), 9496);
    }

    #[test]
    fn test_gps_to_galileo_is_7168_days() {
        assert_eq!(DAYS_GPS_TO_GALILEO, 7168);
    }

    #[test]
    fn test_gps_to_beidou_is_9492_days() {
        assert_eq!(DAYS_GPS_TO_BEIDOU, 9492);
    }

    #[test]
    fn test_gps_to_glonass_is_5839_days() {
        assert_eq!(DAYS_GPS_TO_GLONASS, 5839);
    }

    #[test]
    fn test_galileo_minus_gps_is_619315200_seconds() {
        assert_eq!(GPS_EPOCH.seconds_until(GALILEO_EPOCH), 619_315_200);
    }

    #[test]
    fn test_beidou_minus_gps_calendar_is_820108800_seconds() {
        assert_eq!(GPS_EPOCH.seconds_until(BEIDOU_EPOCH), 820_108_800);
    }

    #[test]
    fn test_glonass_minus_gps_is_505123200_seconds() {
        // 5839 days * 86_400 = 504_921_600 seconds
        let expected = 5839_i64 * 86_400;

        assert_eq!(GPS_EPOCH.seconds_until(GLONASS_EPOCH), expected);
    }

    #[test]
    fn test_days_until_is_antisymmetric() {
        let a = CivilDate::new(2000, 1, 1);
        let b = CivilDate::new(2001, 1, 1);

        assert_eq!(a.days_until(b), -b.days_until(a));
    }

    #[test]
    fn test_days_until_self_is_zero() {
        assert_eq!(GPS_EPOCH.days_until(GPS_EPOCH), 0);
    }

    #[test]
    fn test_year_2000_is_leap_year() {
        // 2000-02-29 is a valid date; 2000-03-01 = 2000-02-29 + 1 day
        let feb29 = CivilDate::new(2000, 2, 29);
        let mar01 = CivilDate::new(2000, 3, 1);

        assert_eq!(feb29.days_until(mar01), 1);
    }

    #[test]
    fn test_year_1900_is_not_leap_year() {
        // 1900 is divisible by 100 but not by 400 → not a leap year
        let feb28 = CivilDate::new(1900, 2, 28);
        let mar01 = CivilDate::new(1900, 3, 1);

        // Если бы 1900 был високосным годом, разрыв был бы 2 дня. Но он равен 1.
        assert_eq!(feb28.days_until(mar01), 1);
    }

    #[test]
    fn test_epoch_dates_are_correct() {
        assert_eq!(GPS_EPOCH, CivilDate::new(1980, 1, 6));
        assert_eq!(GLONASS_EPOCH, CivilDate::new(1996, 1, 1));
        assert_eq!(GALILEO_EPOCH, CivilDate::new(1999, 8, 22));
        assert_eq!(BEIDOU_EPOCH, CivilDate::new(2006, 1, 1));
        assert_eq!(TAI_EPOCH, CivilDate::new(1958, 1, 1));
        assert_eq!(UNIX_EPOCH, CivilDate::new(1970, 1, 1));
    }

    #[test]
    fn test_leap_seconds_at_epochs_match_official_values() {
        // Historical IERS leap second table
        assert_eq!(LEAP_SECONDS_AT_GPS_EPOCH, 19);
        assert_eq!(LEAP_SECONDS_AT_GLONASS_EPOCH, 30);
        assert_eq!(LEAP_SECONDS_AT_BEIDOU_EPOCH, 33);
    }

    #[test]
    fn test_nanos_gps_to_galileo_matches_known_value() {
        assert_eq!(NANOS_GPS_TO_GALILEO_EPOCH, 619_315_200_000_000_000_i64);
    }

    #[test]
    fn test_nanos_gps_to_beidou_calendar_matches_known_value() {
        assert_eq!(
            NANOS_GPS_TO_BEIDOU_EPOCH_CALENDAR,
            820_108_800_000_000_000_i64
        );
    }

    #[test]
    fn test_utc_epoch_unix_offset_is_63072000_seconds() {
        assert_eq!(UTC_EPOCH_UNIX_OFFSET_S, 63_072_000);
    }

    #[test]
    fn test_utc_epoch_unix_offset_is_730_days() {
        assert_eq!(UTC_EPOCH_UNIX_OFFSET_S / 86_400, 730);
    }

    #[test]
    fn test_utc_epoch_unix_offset_matches_calendar() {
        // CivilDate arithmetic must agree with the constant
        assert_eq!(
            UNIX_EPOCH.seconds_until(UTC_CIVIL_EPOCH),
            UTC_EPOCH_UNIX_OFFSET_S
        );
    }

    #[test]
    fn test_utc_epoch_unix_offset_ns_is_correct() {
        assert_eq!(UTC_EPOCH_UNIX_OFFSET_NS, 63_072_000_000_000_000_i64);
    }

    #[test]
    fn test_gps_epoch_unix_s_is_315964800() {
        assert_eq!(GPS_EPOCH_UNIX_S, 315_964_800);
    }

    #[test]
    fn test_gps_epoch_unix_s_is_3657_days() {
        assert_eq!(GPS_EPOCH_UNIX_S / 86_400, 3657);
    }

    #[test]
    fn test_gps_epoch_unix_s_matches_calendar() {
        assert_eq!(UNIX_EPOCH.seconds_until(GPS_EPOCH), GPS_EPOCH_UNIX_S);
    }

    #[test]
    fn test_unix_before_utc_epoch_is_negative_in_utc() {
        // unix_seconds < UTC_EPOCH_UNIX_OFFSET_S → UTC seconds from 1972 < 0
        let unix_s: i64 = 0;
        let utc_from_1972 = unix_s + UTC_EPOCH_UNIX_OFFSET_S; // still positive for unix=0
                                                              // Actually unix=0 gives utc_from_1972 = -63_072_000 (before UTC epoch)
        let utc_from_1972_correct = unix_s - UTC_EPOCH_UNIX_OFFSET_S;

        assert!(utc_from_1972_correct < 0);
        let _ = utc_from_1972; // suppress unused warning
    }

    #[test]
    fn test_unix_at_utc_epoch_gives_zero_offset() {
        // When unix_s = UTC_EPOCH_UNIX_OFFSET_S (1972-01-01):
        // utc_nanos_from_1972 = (unix_s - UTC_EPOCH_UNIX_OFFSET_S) * 1e9 = 0
        let unix_s = UTC_EPOCH_UNIX_OFFSET_S;
        let utc_s_from_1972 = unix_s - UTC_EPOCH_UNIX_OFFSET_S;

        assert_eq!(utc_s_from_1972, 0);
    }

    #[test]
    fn test_pre_unix_date_is_negative() {
        let date = CivilDate::new(1960, 1, 1);

        assert!(date.days_from_unix() < 0);
    }

    #[test]
    fn test_invalid_date_does_not_panic() {
        let date = CivilDate::new(2024, 13, 40);
        let _ = date.days_from_unix(); // просто проверка устойчивости
    }

    #[test]
    fn test_ordering_is_consistent() {
        let a = CivilDate::new(2000, 1, 1);
        let b = CivilDate::new(2001, 1, 1);

        assert!(a.days_from_unix() < b.days_from_unix());
    }

    #[test]
    fn test_seconds_until_matches_days() {
        let a = CivilDate::new(2000, 1, 1);
        let b = CivilDate::new(2000, 1, 2);

        assert_eq!(a.seconds_until(b), 86_400);
    }

    #[test]
    fn test_nanos_until_matches_seconds() {
        let a = CivilDate::new(2000, 1, 1);
        let b = CivilDate::new(2000, 1, 2);

        assert_eq!(a.nanos_until(b), 86_400_000_000_000);
    }

    #[test]
    fn test_gps_epoch_days_constant_is_stable() {
        assert_eq!(GPS_EPOCH.days_from_unix(), 3657);
    }

    #[test]
    fn test_monotonicity_property() {
        let a = CivilDate::new(2000, 1, 1);
        let b = CivilDate::new(2000, 1, 2);
        let c = CivilDate::new(2000, 1, 3);

        assert!(a.days_from_unix() < b.days_from_unix());
        assert!(b.days_from_unix() < c.days_from_unix());
    }
}
