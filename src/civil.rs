//! # Civil date-time representation
//!
//! This module provides [`CivilDateTime`] — a proleptic Gregorian calendar
//! date and time-of-day, derived from a [`Time<Utc>`](crate::Time) value.
//!
//! ## Design
//!
//! - **Pure `no_std`** — no allocation required
//! - **`Display`** outputs ISO 8601 / RFC 3339 format with nanosecond
//!   precision: `2024-01-15T12:34:56.123456789Z`
//! - **Lossless round-trip**: `Time<Utc> -> CivilDateTime → Time<Utc>`
//! - Sub-second precision is preserved to the nanosecond
//!
//! ## Usage
//!
//! ```rust
//! use gnss_time::{Time, Utc};
//!
//! // UTC epoch -> 1972-01-01T00:00:00.000000000Z
//! let dt = Time::<Utc>::EPOCH.to_civil();
//! assert_eq!(dt.year, 1972);
//! assert_eq!(dt.month, 1);
//! assert_eq!(dt.day, 1);
//! assert_eq!(dt.hour, 0);
//! assert_eq!(dt.minute, 0);
//! assert_eq!(dt.second, 0);
//! assert_eq!(dt.nanos, 0);
//!
//! // Display: ISO 8601
//! assert_eq!(dt.to_string(), "1972-01-01T00:00:00.000000000Z");
//! ```
//!
//! ## Epoch notes
//!
//! `Time<Utc>` counts nanoseconds from **1972-01-01 00:00:00 UTC** (the UTC
//! epoch). Dates before 1972 cannot be represented.

use core::fmt;

use crate::{GnssTimeError, Time, Utc};

/// Days from the Unix epoch (1970-01-01) to the UTC epoch (1972-01-01).
///
/// Used as the pivot when converting between `Time<Utc>` nanosecond offsets
/// and proleptic Gregorian calendar dates.
const UTC_EPOCH_DAYS_FROM_UNIX: i64 = 730; // 2 * 365

/// Nanoseconds per day.
const NANOS_PER_DAY: u64 = 86_400 * 1_000_000_000;

/// Nanoseconds per hour.
const NANOS_PER_HOUR: u64 = 3_600 * 1_000_000_000;

/// Nanoseconds per minute.
const NANOS_PER_MINUTE: u64 = 60 * 1_000_000_000;

/// Nanoseconds per second.
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// A proleptic Gregorian calendar date and time-of-day in UTC.
///
/// Produced by [`Time<Utc>::to_civil`](crate::Time::to_civil). Represents a
/// UTC instant as human-readable fields with nanosecond precision.
///
/// ## ISO 8601 / RFC 3339 output
///
/// [`Display`](core::fmt::Display) formats this as:
/// ```text
/// 2024-01-15T12:34:56.123456789Z
/// ```
///
/// The trailing `Z` indicates UTC (no timezone offset).
///
/// ## Range
///
/// The minimum representable date is **1972-01-01 00:00:00 UTC** (the UTC
/// epoch, where `Time<Utc>::EPOCH` corresponds to 0 nanoseconds).
///
/// ## Examples
///
/// ```rust
/// use gnss_time::{Time, Utc};
///
/// let utc = Time::<Utc>::EPOCH;
/// let dt = utc.to_civil();
/// assert_eq!(dt.to_string(), "1972-01-01T00:00:00.000000000Z");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CivilDateTime {
    /// Calendar year (e.g. 2024). Minimum value: 1972.
    pub year: i32,

    /// Calendar month (1–12).
    pub month: u8,

    /// Day of month (1–31).
    pub day: u8,

    /// Hour of day (0–23).
    pub hour: u8,

    /// Minute of hour (0–59).
    pub minute: u8,

    /// Second of minute (0–59).
    ///
    /// Note: leap seconds (value 60) are not representable — GPS time does
    /// not include them, and `Time<Utc>` is a continuous nanosecond counter.
    pub second: u8,

    /// Sub-second nanoseconds (`0–999_999_999`).
    pub nanos: u32,
}

impl CivilDateTime {
    /// Construct from nanoseconds since the UTC epoch (1972-01-01 00:00:00
    /// UTC).
    ///
    /// This is the inverse of [`to_utc_nanos`](Self::to_utc_nanos).
    ///
    /// # Errors
    ///
    /// Returns [`GnssTimeError::Overflow`] if:
    /// - the day count cannot be represented as `i64`
    /// - intermediate calculations overflow
    /// - time components exceed valid ranges
    pub fn from_utc_nanos(nanos: u64) -> Result<Self, GnssTimeError> {
        let days_from_epoch = nanos / NANOS_PER_DAY;
        let rem = nanos % NANOS_PER_DAY;

        // u64 -> i64 SAFE
        let Ok(days_from_epoch_i64) = i64::try_from(days_from_epoch) else {
            return Err(GnssTimeError::Overflow);
        };

        let Some(days_from_unix) = days_from_epoch_i64.checked_add(UTC_EPOCH_DAYS_FROM_UNIX) else {
            return Err(GnssTimeError::Overflow);
        };

        let (year, month, day) = civil_from_days(days_from_unix);

        let Ok(hour) = u8::try_from(rem / NANOS_PER_HOUR) else {
            return Err(GnssTimeError::Overflow);
        };

        let Ok(minute) = u8::try_from((rem % NANOS_PER_HOUR) / NANOS_PER_MINUTE) else {
            return Err(GnssTimeError::Overflow);
        };

        let Ok(second) = u8::try_from((rem % NANOS_PER_MINUTE) / NANOS_PER_SECOND) else {
            return Err(GnssTimeError::Overflow);
        };

        let Ok(nanos) = u32::try_from(rem % NANOS_PER_SECOND) else {
            return Err(GnssTimeError::Overflow);
        };

        Ok(CivilDateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanos,
        })
    }

    /// Returns nanoseconds since the UTC epoch (1972-01-01 00:00:00 UTC).
    ///
    /// This is the inverse of [`from_utc_nanos`](Self::from_utc_nanos).
    ///
    /// # Errors
    ///
    /// Returns [`GnssTimeError::Overflow`] if:
    /// - the date is before the UTC epoch (1972-01-01)
    /// - intermediate arithmetic overflows
    /// - the resulting nanosecond count does not fit in `u64`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gnss_time::CivilDateTime;
    ///
    /// let dt = CivilDateTime::from_utc_nanos(0).unwrap();
    /// assert_eq!(dt.to_utc_nanos().unwrap(), 0);
    ///
    /// let dt2 = CivilDateTime::from_utc_nanos(1_234_567_890_123_456_789).unwrap();
    /// assert_eq!(dt2.to_utc_nanos().unwrap(), 1_234_567_890_123_456_789);
    /// ```
    #[must_use = "conversion errors must be handled"]
    pub fn to_utc_nanos(self) -> Result<u64, GnssTimeError> {
        let days_from_unix = days_to_unix(self.year, self.month, self.day);

        let days_from_epoch = days_from_unix
            .checked_sub(UTC_EPOCH_DAYS_FROM_UNIX)
            .ok_or(GnssTimeError::Overflow)?;

        let days_u64 = u64::try_from(days_from_epoch).map_err(|_| GnssTimeError::Overflow)?;

        let day_ns = days_u64
            .checked_mul(NANOS_PER_DAY)
            .ok_or(GnssTimeError::Overflow)?;

        let time_ns = u64::from(self.hour)
            .checked_mul(NANOS_PER_HOUR)
            .and_then(|v| v.checked_add(u64::from(self.minute) * NANOS_PER_MINUTE))
            .and_then(|v| v.checked_add(u64::from(self.second) * NANOS_PER_SECOND))
            .and_then(|v| v.checked_add(u64::from(self.nanos)))
            .ok_or(GnssTimeError::Overflow)?;

        day_ns.checked_add(time_ns).ok_or(GnssTimeError::Overflow)
    }

    /// Converts this civil date-time to a [`Time<Utc>`].
    ///
    /// # Errors
    ///
    /// Returns [`GnssTimeError::Overflow`] if the date is before the UTC epoch
    /// (1972-01-01) or if the nanosecond count overflows `u64`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gnss_time::{CivilDateTime, Time, Utc};
    ///
    /// let dt = CivilDateTime::from_utc_nanos(0).unwrap();
    /// let utc = dt.to_utc().unwrap();
    /// assert_eq!(utc, Time::<Utc>::EPOCH);
    /// ```
    pub fn to_utc(self) -> Result<Time<Utc>, GnssTimeError> {
        let nanos = self.to_utc_nanos()?;

        Ok(Time::<Utc>::from_nanos(nanos))
    }

    /// Returns `true` if the sub-second part is zero (whole second).
    #[inline]
    #[must_use]
    pub const fn is_whole_second(&self) -> bool {
        self.nanos == 0
    }
}

/// Converts days since Unix epoch (1970-01-01) to a proleptic Gregorian date.
///
/// Implementation of Howard Hinnant's `civil_from_days` algorithm:
/// <http://howardhinnant.github.io/date_algorithms.html>
#[must_use]
fn civil_from_days(z: i64) -> (i32, u8, u8) {
    let z = z + 719_468;

    let era: i64 = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };

    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);

    let mp = (5 * doy + 2) / 153;
    let day_i64 = doy - (153 * mp + 2) / 5 + 1;
    let month_i64 = if mp < 10 { mp + 3 } else { mp - 9 };

    let year_i64 = if month_i64 <= 2 { y + 1 } else { y };

    let year = i32::try_from(year_i64).unwrap_or(i32::MAX);
    let month = u8::try_from(month_i64).unwrap_or_else(|_| unreachable!("month out of range"));
    let day = u8::try_from(day_i64).unwrap_or_else(|_| unreachable!("day out of range"));

    (year, month, day)
}

/// Converts a proleptic Gregorian date to days since Unix epoch (1970-01-01).
///
/// Implementation of Howard Hinnant's `days_from_civil` algorithm.
fn days_to_unix(
    year: i32,
    month: u8,
    day: u8,
) -> i64 {
    let y = if month <= 2 {
        i64::from(year) - 1
    } else {
        i64::from(year)
    };
    let m = if month <= 2 {
        i64::from(month) + 9
    } else {
        i64::from(month) - 3
    };
    let d = i64::from(day);

    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = y - era * 400; // [0, 399]
    let doy = (153 * m + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]

    era * 146_097 + doe - 719_468
}

impl fmt::Display for CivilDateTime {
    /// Formats as ISO 8601 / RFC 3339 with nanosecond precision.
    ///
    /// Format: `YYYY-MM-DDThh:mm:ss.nnnnnnnnnZ`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gnss_time::CivilDateTime;
    ///
    /// let dt = CivilDateTime::from_utc_nanos(0).unwrap();
    /// assert_eq!(dt.to_string(), "1972-01-01T00:00:00.000000000Z");
    /// ```
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.nanos,
        )
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use std::string::ToString;

    use super::*;
    use crate::{Time, Utc};

    #[test]
    fn test_utc_epoch_nanos_zero_gives_1972_01_01() {
        let dt = CivilDateTime::from_utc_nanos(0).unwrap();

        assert_eq!(dt.year, 1972);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
        assert_eq!(dt.nanos, 0);
    }

    #[test]
    fn test_gps_epoch_as_utc_nanos() {
        // GPS epoch = 1980-01-06 = 2927 days from UTC epoch
        // = 2927 * 86_400_000_000_000 nanoseconds
        let nanos = 252_892_800_000_000_000_u64;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.year, 1980);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 6);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
        assert_eq!(dt.nanos, 0);
    }

    #[test]
    fn test_2017_01_01_from_utc_nanos() {
        // 2017-01-01 00:00:00 UTC = 16437 days from UTC epoch
        let nanos = 16_437_u64 * 86_400 * 1_000_000_000;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.year, 2017);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
    }

    #[test]
    fn test_sub_second_precision() {
        // 1972-01-01T00:00:00.123456789Z
        let dt = CivilDateTime::from_utc_nanos(123_456_789).unwrap();

        assert_eq!(dt.year, 1972);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
        assert_eq!(dt.nanos, 123_456_789);
    }

    #[test]
    fn test_time_of_day_decomposition() {
        // 1972-01-01T12:34:56.000000000Z
        let h: u64 = 12;
        let m: u64 = 34;
        let s: u64 = 56;
        let nanos = h * 3_600 * 1_000_000_000 + m * 60 * 1_000_000_000 + s * 1_000_000_000;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.hour, 12);
        assert_eq!(dt.minute, 34);
        assert_eq!(dt.second, 56);
        assert_eq!(dt.nanos, 0);
    }

    #[test]
    fn test_2024_01_15_t12_34_56_with_nanos() {
        // 2024-01-15T12:34:56.123456789Z
        // 2024-01-15 is 19007 days from UTC epoch (verified in Python)
        let day_ns: u64 = 19_007 * 86_400 * 1_000_000_000;
        let time_ns: u64 =
            12 * 3_600 * 1_000_000_000 + 34 * 60 * 1_000_000_000 + 56 * 1_000_000_000 + 123_456_789;
        let dt = CivilDateTime::from_utc_nanos(day_ns + time_ns).unwrap();

        assert_eq!(dt.year, 2024);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 15);
        assert_eq!(dt.hour, 12);
        assert_eq!(dt.minute, 34);
        assert_eq!(dt.second, 56);
        assert_eq!(dt.nanos, 123_456_789);
    }

    #[test]
    fn test_leap_year_feb_29() {
        // 2000 is a leap year; 2000-02-29 must parse correctly
        // Days from 1972-01-01 to 2000-02-29:
        // 1972 -> 2000: 28 years, with leap years 1972,1976,...,2000 -> 7 leap
        // (28*365 + 7) - 1 = 10218 days from 1972-01-01 (0-indexed)
        // Actually let's compute: days_from_unix(2000,2,29) - 730
        // days_from_unix(2000,2,29) = 11_016 (verified)
        let days_from_utc_epoch: u64 = 11_016 - 730;
        let nanos = days_from_utc_epoch * 86_400 * 1_000_000_000;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.year, 2000);
        assert_eq!(dt.month, 2);
        assert_eq!(dt.day, 29);
    }

    #[test]
    fn test_last_day_of_year() {
        // 1972-12-31T23:59:59.999999999Z
        // 1972-12-31 is 365 days from 1972-01-01 -> day index 365 from epoch
        // But 1972 is a leap year, so 366 days total.
        // 1972-12-31 = day 365 (0-indexed from Jan 1)
        let days: u64 = 365; // 1972-12-31 from 1972-01-01
        let nanos = days * 86_400 * 1_000_000_000
            + 23 * 3_600 * 1_000_000_000
            + 59 * 60 * 1_000_000_000
            + 59 * 1_000_000_000
            + 999_999_999;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.year, 1972);
        assert_eq!(dt.month, 12);
        assert_eq!(dt.day, 31);
        assert_eq!(dt.hour, 23);
        assert_eq!(dt.minute, 59);
        assert_eq!(dt.second, 59);
        assert_eq!(dt.nanos, 999_999_999);
    }

    #[test]
    fn test_roundtrip_epoch() {
        let nanos: u64 = 0;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.to_utc_nanos().unwrap(), nanos);
    }

    #[test]
    fn test_roundtrip_gps_epoch() {
        let nanos: u64 = 252_892_800_000_000_000;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.to_utc_nanos().unwrap(), nanos);
    }

    #[test]
    fn test_roundtrip_with_sub_second() {
        let nanos: u64 = 1_234_567_890_123_456_789;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.to_utc_nanos().unwrap(), nanos);
    }

    #[test]
    fn test_roundtrip_many_values() {
        let cases: &[u64] = &[
            0,
            1,
            999_999_999,
            1_000_000_000,
            86_400_000_000_000,        // 1 day
            252_892_800_000_000_000,   // GPS epoch
            1_420_156_800_000_000_000, // 2017-01-01
            1_642_204_800_000_000_000, // 2022-01-15
        ];

        for &n in cases {
            let dt = CivilDateTime::from_utc_nanos(n).unwrap();

            assert_eq!(
                dt.to_utc_nanos().unwrap(),
                n,
                "round-trip failed for nanos={n}"
            );
        }
    }

    #[test]
    fn test_to_utc_epoch() {
        let dt = CivilDateTime::from_utc_nanos(0).unwrap();
        let utc = dt.to_utc().unwrap();

        assert_eq!(utc, Time::<Utc>::EPOCH);
    }

    #[test]
    fn test_to_utc_gps_epoch_date() {
        let dt = CivilDateTime {
            year: 1980,
            month: 1,
            day: 6,
            hour: 0,
            minute: 0,
            second: 0,
            nanos: 0,
        };
        let utc = dt.to_utc().unwrap();

        assert_eq!(utc.as_nanos(), 252_892_800_000_000_000);
    }

    #[test]
    fn test_to_utc_before_1972_fails() {
        let dt = CivilDateTime {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            nanos: 0,
        };

        assert!(matches!(dt.to_utc(), Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_time_utc_epoch_to_civil() {
        let dt = Time::<Utc>::EPOCH.to_civil();

        assert_eq!(dt.year, 1972);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
        assert_eq!(dt.nanos, 0);
    }

    #[test]
    fn test_time_utc_to_civil_roundtrip() {
        let original = Time::<Utc>::from_nanos(1_234_567_890_123_456_789);
        let dt = original.to_civil();
        let back = dt.to_utc().unwrap();

        assert_eq!(original, back);
    }

    #[test]
    fn test_time_utc_from_unix_to_civil() {
        // 2024-01-01 00:00:00 UTC = Unix 1_704_067_200
        let utc = Time::<Utc>::from_unix_seconds(1_704_067_200).unwrap();
        let dt = utc.to_civil();

        assert_eq!(dt.year, 2024);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.minute, 0);
        assert_eq!(dt.second, 0);
    }

    #[test]
    fn test_display_utc_epoch() {
        let dt = CivilDateTime::from_utc_nanos(0).unwrap();

        assert_eq!(dt.to_string(), "1972-01-01T00:00:00.000000000Z");
    }

    #[test]
    fn test_display_gps_epoch() {
        let dt = CivilDateTime::from_utc_nanos(252_892_800_000_000_000).unwrap();

        assert_eq!(dt.to_string(), "1980-01-06T00:00:00.000000000Z");
    }

    #[test]
    fn test_display_with_time_and_sub_second() {
        let dt = CivilDateTime {
            year: 2024,
            month: 1,
            day: 15,
            hour: 12,
            minute: 34,
            second: 56,
            nanos: 123_456_789,
        };

        assert_eq!(dt.to_string(), "2024-01-15T12:34:56.123456789Z");
    }

    #[test]
    fn test_display_zero_padded_month_day() {
        let dt = CivilDateTime {
            year: 1972,
            month: 3,
            day: 5,
            hour: 1,
            minute: 2,
            second: 3,
            nanos: 0,
        };

        assert_eq!(dt.to_string(), "1972-03-05T01:02:03.000000000Z");
    }

    #[test]
    fn test_display_ends_with_z() {
        let dt = CivilDateTime::from_utc_nanos(0).unwrap();

        assert!(dt.to_string().ends_with('Z'));
    }

    #[test]
    fn test_display_contains_t_separator() {
        let dt = CivilDateTime::from_utc_nanos(0).unwrap();

        assert!(dt.to_string().contains('T'));
    }

    #[test]
    fn test_display_format_length() {
        // "YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ" = 30 characters
        let dt = CivilDateTime::from_utc_nanos(0).unwrap();

        assert_eq!(dt.to_string().len(), 30);
    }

    #[test]
    fn test_is_whole_second_true() {
        let dt = CivilDateTime::from_utc_nanos(1_000_000_000).unwrap();

        assert!(dt.is_whole_second());
    }

    #[test]
    fn test_is_whole_second_false() {
        let dt = CivilDateTime::from_utc_nanos(1_000_000_001).unwrap();

        assert!(!dt.is_whole_second());
    }

    #[test]
    fn test_year_2000_leap_year() {
        // 2000-02-29 exists (2000 is a leap year)
        let days: u64 = (11_016 - 730) as u64; // pre-verified
        let nanos = days * 86_400 * 1_000_000_000;
        let dt = CivilDateTime::from_utc_nanos(nanos).unwrap();

        assert_eq!(dt.year, 2000);
        assert_eq!(dt.month, 2);
        assert_eq!(dt.day, 29);

        // And the day after is 2000-03-01
        let next = CivilDateTime::from_utc_nanos(nanos + 86_400_000_000_000).unwrap();
        assert_eq!(next.year, 2000);
        assert_eq!(next.month, 3);
        assert_eq!(next.day, 1);
    }

    #[test]
    fn test_year_1900_not_leap_year() {
        // 1900 is NOT a leap year (divisible by 100 but not 400)
        // 1900-02-28 -> next day must be 1900-03-01, not 1900-02-29
        // We can't represent 1900 (before UTC epoch), but we can verify
        // the algorithm handles it correctly via days_to_unix / civil_from_days
        let days_1900_feb28 = super::days_to_unix(1900, 2, 28);
        let (y, m, d) = super::civil_from_days(days_1900_feb28 + 1);

        assert_eq!((y, m, d), (1900, 3, 1));
    }

    #[test]
    fn test_midnight_boundary() {
        // Last nanosecond of a day, and the first of the next
        let day_ns = 86_400_000_000_000_u64;
        let end_of_day = CivilDateTime::from_utc_nanos(day_ns - 1).unwrap();
        let start_of_next = CivilDateTime::from_utc_nanos(day_ns).unwrap();

        assert_eq!(end_of_day.hour, 23);
        assert_eq!(end_of_day.minute, 59);
        assert_eq!(end_of_day.second, 59);
        assert_eq!(end_of_day.nanos, 999_999_999);
        assert_eq!(start_of_next.day, end_of_day.day + 1);
        assert_eq!(start_of_next.hour, 0);
    }
}
