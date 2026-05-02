//! # Unified type-safe conversion API
//!
//! This module provides the [`IntoScale`] and [`IntoScaleWith`] traits for
//! converting between GNSS time scales, plus [`ConvertResult`] for handling the
//! one-second ambiguity window that can occur during leap-second insertion.
//!
//! ## Conversion overview
//!
//! | From → To       | API                                | Leap seconds?          |
//! |-----------------|------------------------------------|------------------------|
//! | `Galileo → GPS` | [`IntoScale::into_scale`]          | No                     |
//! | `GPS → TAI`     | [`IntoScale::into_scale`]          | No                     |
//! | `GPS → Galileo` | [`IntoScale::into_scale`]          | No                     |
//! | `GPS → BeiDou`  | [`IntoScale::into_scale`]          | No                     |
//! | `BeiDou → GPS`  | [`IntoScale::into_scale`]          | No                     |
//! | `GPS → UTC`     | [`IntoScaleWith::into_scale_with`] | Yes                    |
//! | `UTC → GPS`     | [`IntoScaleWith::into_scale_with`] | Yes                    |
//! | `GLO → UTC`     | [`IntoScale::into_scale`]          | No (fixed epoch shift) |
//! | `UTC → GLO`     | [`IntoScale::into_scale`]          | No (fixed epoch shift) |
//! | `GPS → GLO`     | [`IntoScaleWith::into_scale_with`] | Yes                    |
//! | `GLO → GPS`     | [`IntoScaleWith::into_scale_with`] | Yes                    |
//!
//! ## Usage
//!
//! Fixed-offset conversions do not require leap seconds:
//!
//! ```rust
//! use gnss_time::{DurationParts, Galileo, Gps, IntoScale, Tai, Time};
//!
//! let gps = Time::<Gps>::from_week_tow(
//!     2345,
//!     DurationParts {
//!         seconds: 0,
//!         nanos: 0,
//!     },
//! )
//! .unwrap();
//! let tai: Time<Tai> = gps.into_scale().unwrap();
//! let gal: Time<Galileo> = gps.into_scale().unwrap();
//! ```
//!
//! Leap-second-aware conversions require an explicit [`LeapSecondsProvider`]:
//!
//! ```rust
//! use gnss_time::{DurationParts, Gps, IntoScaleWith, LeapSeconds, Time, Utc};
//!
//! let gps = Time::<Gps>::from_week_tow(
//!     2200,
//!     DurationParts {
//!         seconds: 0,
//!         nanos: 0,
//!     },
//! )
//! .unwrap();
//! let ls = LeapSeconds::builtin();
//! let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
//! ```
//!
//! ## Leap-second ambiguity
//!
//! During leap-second insertion, `GPS → UTC` may map to a one-second window
//! that cannot be represented as a single unambiguous civil-time value.
//! Use [`IntoScaleWith::into_scale_with_checked`] to detect that case:
//!
//! ```rust
//! use gnss_time::{ConvertResult, Gps, IntoScaleWith, LeapSeconds, Time, Utc};
//!
//! let ls = LeapSeconds::builtin();
//! let gps = Time::<Gps>::from_seconds(1_167_264_018);
//!
//! let result: Result<ConvertResult<Time<Utc>>, _> = gps.into_scale_with_checked(ls);
//!
//! assert!(matches!(result, Ok(ConvertResult::AmbiguousLeapSecond(_))));
//! ```

use crate::{
    beidou_to_glonass, beidou_to_utc, galileo_to_glonass, galileo_to_utc, glonass_to_beidou,
    glonass_to_galileo, glonass_to_gps, glonass_to_utc, gps_to_glonass, gps_to_utc, utc_to_beidou,
    utc_to_galileo, utc_to_glonass, utc_to_gps, Beidou, Galileo, Glonass, GnssTimeError, Gps,
    LeapSecondsProvider, Tai, Time, TimeScale, Utc,
};

/// Converts `Time<Self>` into `Time<Target>` using a fixed offset.
///
/// This trait is implemented only for conversions whose offset is known at
/// compile time.
///
/// Typical examples:
/// - `GPS ↔ TAI`
/// - `GPS ↔ Galileo`
/// - `GPS ↔ BeiDou`
/// - `GLONASS ↔ UTC`
///
/// # Errors
///
/// Returns [`GnssTimeError::Overflow`] if the converted value does not fit into
/// the destination time representation.
pub trait IntoScale<Target: TimeScale>: Sized {
    /// Converts using a fixed offset.
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}

/// Converts `Time<Self>` into `Time<Target>` using an explicit leap-second
/// table.
///
/// This trait is required for conversions that depend on civil time, such as
/// `UTC ↔ GPS` and `GLONASS ↔ GPS`.
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    /// Converts using the provided leap-second source.
    ///
    /// Returns [`GnssTimeError::Overflow`] if the result cannot be represented.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Target>, GnssTimeError>;

    /// Converts using the provided leap-second source and reports leap-second
    /// ambiguity when applicable.
    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}

/// Result of a conversion that may be ambiguous during leap-second insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertResult<T> {
    /// The conversion is fully unambiguous.
    Exact(T),

    /// The source time falls inside a leap-second window.
    ///
    /// The inner value is the closest representable UTC timestamp.
    AmbiguousLeapSecond(T),
}

impl<T> ConvertResult<T> {
    /// Returns the inner value regardless of the variant.
    #[inline]
    pub fn into_inner(self) -> T {
        match self {
            ConvertResult::Exact(t) | ConvertResult::AmbiguousLeapSecond(t) => t,
        }
    }

    /// Returns `true` if the result is exact.
    #[inline]
    pub fn is_exact(&self) -> bool {
        matches!(self, ConvertResult::Exact(_))
    }

    /// Returns `true` if the result is ambiguous.
    #[inline]
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, ConvertResult::AmbiguousLeapSecond(_))
    }
}

////////////////////////////////////////////////////////////////////////////////
// GLONASS for Gps, Galileo, Beidou, UTC
////////////////////////////////////////////////////////////////////////////////

impl IntoScale<Glonass> for Time<Utc> {
    /// UTC -> GLONASS: постоянный сдвиг эпохи.
    ///
    /// # Errors
    ///
    /// [`GnssTimeError::Overflow`] если UTC раньше эпохи GLONASS
    /// (1995-12-31 21:00:00 UTC).
    #[inline]
    fn into_scale(self) -> Result<Time<Glonass>, GnssTimeError> {
        utc_to_glonass(self)
    }
}

impl IntoScaleWith<Glonass> for Time<Gps> {
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Glonass>, GnssTimeError> {
        gps_to_glonass(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Glonass>>, GnssTimeError> {
        Ok(ConvertResult::Exact(gps_to_glonass(self, &ls)?))
    }
}

impl IntoScaleWith<Glonass> for Time<Galileo> {
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Glonass>, GnssTimeError> {
        galileo_to_glonass(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Glonass>>, GnssTimeError> {
        Ok(ConvertResult::Exact(galileo_to_glonass(self, &ls)?))
    }
}

impl IntoScaleWith<Glonass> for Time<Beidou> {
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Glonass>, GnssTimeError> {
        beidou_to_glonass(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Glonass>>, GnssTimeError> {
        Ok(ConvertResult::Exact(beidou_to_glonass(self, &ls)?))
    }
}

////////////////////////////////////////////////////////////////////////////////
// Gps for Glonass, Galileo, Beidou, Tai, Utc
////////////////////////////////////////////////////////////////////////////////

impl IntoScale<Gps> for Time<Galileo> {
    #[inline]
    fn into_scale(self) -> Result<Time<Gps>, GnssTimeError> {
        self.try_convert::<Gps>()
    }
}

impl IntoScale<Gps> for Time<Beidou> {
    /// BeiDou -> GPS: `GPS = BDT + 14s`.
    ///
    /// ```rust
    /// use gnss_time::{Beidou, Gps, IntoScale, Time};
    ///
    /// let bdt = Time::<Beidou>::from_seconds(86);
    /// let gps: Time<Gps> = bdt.into_scale().unwrap();
    ///
    /// assert_eq!(gps.as_seconds(), 100); // 86 - 19 + 33 = 100
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Gps>, GnssTimeError> {
        self.try_convert::<Gps>()
    }
}

impl IntoScale<Gps> for Time<Tai> {
    /// TAI -> GPS: subtract 19 seconds.
    ///
    /// ```rust
    /// use gnss_time::{Gps, IntoScale, Tai, Time};
    ///
    /// let tai = Time::<Tai>::from_seconds(119);
    /// let gps: Time<Gps> = tai.into_scale().unwrap();
    ///
    /// assert_eq!(gps.as_seconds(), 100);
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Gps>, GnssTimeError> {
        Time::<Gps>::from_tai(self)
    }
}

impl IntoScaleWith<Gps> for Time<Glonass> {
    /// GLONASS -> GPS via UTC.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Gps>, GnssTimeError> {
        glonass_to_gps(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Gps>>, GnssTimeError> {
        Ok(ConvertResult::Exact(glonass_to_gps(self, &ls)?))
    }
}

impl IntoScaleWith<Gps> for Time<Utc> {
    /// UTC -> GPS with leap-second context.
    ///
    /// ```rust
    /// use gnss_time::{DurationParts, Gps, IntoScale, IntoScaleWith, LeapSeconds, Time, Utc};
    ///
    /// let ls = LeapSeconds::builtin();
    /// let gps_orig = Time::<Gps>::from_week_tow(
    ///     2086,
    ///     DurationParts {
    ///         seconds: 0,
    ///         nanos: 0,
    ///     },
    /// )
    /// .unwrap();
    /// let utc: Time<Utc> = gps_orig.into_scale_with(ls).unwrap();
    /// let gps_back: Time<Gps> = utc.into_scale_with(ls).unwrap();
    ///
    /// assert_eq!(gps_orig, gps_back);
    /// ```
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Gps>, GnssTimeError> {
        utc_to_gps(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Gps>>, GnssTimeError> {
        // UTC -> GPS is unambiguous: each UTC nanosecond corresponds to
        // exactly one GPS nanosecond (GPS has no skipped or repeated seconds).
        Ok(ConvertResult::Exact(utc_to_gps(self, &ls)?))
    }
}

////////////////////////////////////////////////////////////////////////////////
// Galileo for Glonass, Gps, Beidou, Utc
////////////////////////////////////////////////////////////////////////////////

impl IntoScale<Galileo> for Time<Gps> {
    /// GPS -> Galileo: identical at nanosecond level (both use `TAI − 19s`).
    ///
    /// GPS and Galileo timestamps with identical nanoseconds represent
    /// the same physical instant.
    ///
    /// ```rust
    /// use gnss_time::{Galileo, Gps, IntoScale, Time};
    ///
    /// let gps = Time::<Gps>::from_seconds(12_345);
    /// let gal: Time<Galileo> = gps.into_scale().unwrap();
    ///
    /// assert_eq!(gps.as_nanos(), gal.as_nanos());
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Galileo>, GnssTimeError> {
        // GPS and Galileo use the same offset relative to TAI (19 s)
        // → converting via TAI preserves nanoseconds exactly
        self.try_convert::<Galileo>()
    }
}

impl IntoScale<Galileo> for Time<Beidou> {
    /// BeiDou -> Galileo via TAI.
    #[inline]
    fn into_scale(self) -> Result<Time<Galileo>, GnssTimeError> {
        self.try_convert::<Galileo>()
    }
}

impl IntoScaleWith<Galileo> for Time<Glonass> {
    /// GLONASS -> Galileo via UTC.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Galileo>, GnssTimeError> {
        glonass_to_galileo(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Galileo>>, GnssTimeError> {
        Ok(ConvertResult::Exact(glonass_to_galileo(self, &ls)?))
    }
}

impl IntoScaleWith<Galileo> for Time<Utc> {
    /// UTC -> Galileo via GPS.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Galileo>, GnssTimeError> {
        utc_to_galileo(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Galileo>>, GnssTimeError> {
        Ok(ConvertResult::Exact(utc_to_galileo(self, &ls)?))
    }
}

////////////////////////////////////////////////////////////////////////////////
// Beidou for Glonass, Gps, Galileo, Utc
////////////////////////////////////////////////////////////////////////////////

impl IntoScale<Beidou> for Time<Gps> {
    /// GPS -> BeiDou: `BDT = GPS - 14s`.
    ///
    /// ```rust
    /// use gnss_time::{Beidou, Gps, IntoScale, Time};
    ///
    /// let gps = Time::<Gps>::from_seconds(100);
    /// let bdt: Time<Beidou> = gps.into_scale().unwrap();
    ///
    /// assert_eq!(bdt.as_seconds(), 86); // 100 - 14 = 86
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Beidou>, GnssTimeError> {
        self.try_convert::<Beidou>()
    }
}

impl IntoScale<Beidou> for Time<Galileo> {
    /// Galileo -> BeiDou via TAI.
    #[inline]
    fn into_scale(self) -> Result<Time<Beidou>, GnssTimeError> {
        self.try_convert::<Beidou>()
    }
}

impl IntoScaleWith<Beidou> for Time<Utc> {
    /// UTC → BeiDou via GPS.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Beidou>, GnssTimeError> {
        utc_to_beidou(self, &ls)
    }
    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Beidou>>, GnssTimeError> {
        Ok(ConvertResult::Exact(utc_to_beidou(self, &ls)?))
    }
}

impl IntoScaleWith<Beidou> for Time<Glonass> {
    /// GLONASS -> BeiDou via UTC.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Beidou>, GnssTimeError> {
        glonass_to_beidou(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Beidou>>, GnssTimeError> {
        Ok(ConvertResult::Exact(glonass_to_beidou(self, &ls)?))
    }
}

////////////////////////////////////////////////////////////////////////////////
// Utc for Glonass, Gps, Galileo, Beidou
////////////////////////////////////////////////////////////////////////////////

impl IntoScale<Utc> for Time<Glonass> {
    /// GLONASS -> UTC: fixed epoch shift.
    ///
    /// GLONASS tracks UTC(SU) = UTC + 3 hours including leap seconds.
    /// Therefore conversion is a pure epoch offset.
    ///
    /// ```rust
    /// use gnss_time::{DurationParts, Glonass, IntoScale, Time, Utc};
    ///
    /// let glo = Time::<Glonass>::from_day_tod(
    ///     0,
    ///     DurationParts {
    ///         seconds: 0,
    ///         nanos: 0,
    ///     },
    /// )
    /// .unwrap(); // GLONASS epoch
    /// let utc: Time<Utc> = glo.into_scale().unwrap();
    ///
    /// // UTC at the GLONASS epoch:
    /// // 1995-12-31 21:00:00 UTC = 757_371_600 s from 1972
    /// assert_eq!(utc.as_nanos(), 757_371_600_000_000_000);
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Utc>, GnssTimeError> {
        glonass_to_utc(self)
    }
}

impl IntoScaleWith<Utc> for Time<Gps> {
    /// GPS -> UTC with leap-second context.
    ///
    /// Round-trip consistency: `GPS -> UTC -> GPS` is exact (< 1 ns) for all
    /// moments except the one-second leap-second insertion window.
    ///
    /// ```rust
    /// use gnss_time::{Gps, IntoScaleWith, LeapSeconds, Time, Utc};
    ///
    /// let ls = LeapSeconds::builtin();
    /// let gps = Time::<Gps>::from_seconds(1_167_264_018); // 2017-01-01 GPS
    /// let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
    ///
    /// let delta = gps.as_seconds() as i64 - utc.as_seconds() as i64 + 252_892_800_i64;
    ///
    /// // GPS leads UTC by 18 s → UTC is 18 s earlier
    /// assert_eq!(delta, 18);
    /// ```
    #[inline]
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Utc>, GnssTimeError> {
        gps_to_utc(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Utc>>, GnssTimeError> {
        let utc = gps_to_utc(self, &ls)?;

        // Detect leap-second window: compute TAI at this GPS timestamp
        // and compare leap-second offsets before and after.
        // If values differ — we are inside (or adjacent to) a leap-second boundary.
        let tai = self.to_tai()?;
        let n_at = ls.tai_minus_utc_at(tai);

        // Check 1 second back to detect entry into leap second
        let tai_prev = if tai.as_nanos() >= 1_000_000_000 {
            Time::<Tai>::from_nanos(tai.as_nanos() - 1_000_000_000)
        } else {
            tai
        };
        let n_before = ls.tai_minus_utc_at(tai_prev);

        if n_at != n_before {
            // We crossed a leap-second boundary within the last second.
            // The GPS second corresponding to the old offset is ambiguous.
            Ok(ConvertResult::AmbiguousLeapSecond(utc))
        } else {
            Ok(ConvertResult::Exact(utc))
        }
    }
}

impl IntoScaleWith<Utc> for Time<Galileo> {
    /// Galileo -> UTC via GPS (both share the same TAI offset of 19s).
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Utc>, GnssTimeError> {
        galileo_to_utc(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Utc>>, GnssTimeError> {
        Ok(ConvertResult::Exact(galileo_to_utc(self, &ls)?))
    }
}

impl IntoScaleWith<Utc> for Time<Beidou> {
    /// BeiDou -> UTC via GPS.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Utc>, GnssTimeError> {
        beidou_to_utc(self, &ls)
    }

    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Utc>>, GnssTimeError> {
        Ok(ConvertResult::Exact(beidou_to_utc(self, &ls)?))
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tai for Gps
////////////////////////////////////////////////////////////////////////////////

impl IntoScale<Tai> for Time<Gps> {
    /// GPS -> TAI: add 19 seconds (constant, no leap seconds).
    ///
    /// ```rust
    /// use gnss_time::{Gps, IntoScale, Tai, Time};
    ///
    /// let gps = Time::<Gps>::from_seconds(100);
    /// let tai: Time<Tai> = gps.into_scale().unwrap();
    ///
    /// assert_eq!(tai.as_seconds(), 119); // 100 + 19
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Tai>, GnssTimeError> {
        self.to_tai()
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DurationParts, LeapSeconds};

    #[test]
    fn test_gps_to_tai_adds_19_seconds() {
        let gps = Time::<Gps>::from_seconds(100);
        let tai: Time<Tai> = gps.into_scale().unwrap();

        // 100 + 19
        assert_eq!(tai.as_seconds(), 119);
    }

    #[test]
    fn test_tai_to_gps_subtracts_19_seconds() {
        let tai = Time::<Tai>::from_seconds(119);
        let gps: Time<Gps> = tai.into_scale().unwrap();

        // 119 - 19 = 100
        assert_eq!(gps.as_seconds(), 100);
    }

    #[test]
    fn test_gps_tai_gps_roundtrip() {
        let gps = Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap();
        let tai: Time<Tai> = gps.into_scale().unwrap();
        let back: Time<Gps> = tai.into_scale().unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_tai_to_gps_underflow_at_tai_zero() {
        // TAI(0) − 19 s → negative GPS time → overflow
        let tai = Time::<Tai>::EPOCH;
        let result: Result<Time<Gps>, _> = tai.into_scale();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_gps_to_galileo_preserves_nanos() {
        let gps = Time::<Gps>::from_seconds(12_345_678);
        let gal: Time<Galileo> = gps.into_scale().unwrap();

        assert_eq!(gps.as_nanos(), gal.as_nanos());
    }

    #[test]
    fn test_galileo_to_gps_preserves_nanos() {
        let gal = Time::<Galileo>::from_seconds(99_999_999);
        let gps: Time<Gps> = gal.into_scale().unwrap();

        assert_eq!(gal.as_nanos(), gps.as_nanos());
    }

    #[test]
    fn test_gps_galileo_gps_roundtrip() {
        let gps = Time::<Gps>::from_week_tow(
            2000,
            DurationParts {
                seconds: 123_456,
                nanos: 789_000_000,
            },
        )
        .unwrap();
        let gal: Time<Galileo> = gps.into_scale().unwrap();
        let back: Time<Gps> = gal.into_scale().unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_to_beidou_subtracts_14_seconds() {
        // GPS + 19 s = TAI; BDT + 33 s = TAI → BDT = GPS + 19 - 33 = GPS - 14
        let gps = Time::<Gps>::from_seconds(100);
        let bdt: Time<Beidou> = gps.into_scale().unwrap();

        assert_eq!(bdt.as_seconds(), 86); // 100 - 14 = 86
    }

    #[test]
    fn test_beidou_to_gps_adds_14_seconds() {
        let bdt = Time::<Beidou>::from_seconds(86);
        let gps: Time<Gps> = bdt.into_scale().unwrap();

        assert_eq!(gps.as_seconds(), 100);
    }

    #[test]
    fn test_gps_beidou_gps_roundtrip() {
        let gps = Time::<Gps>::from_week_tow(
            2100,
            DurationParts {
                seconds: 86_400,
                nanos: 0,
            },
        )
        .unwrap();
        let bdt: Time<Beidou> = gps.into_scale().unwrap();
        let back: Time<Gps> = bdt.into_scale().unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_galileo_beidou_roundtrip() {
        let gal = Time::<Galileo>::from_seconds(1_000_000_000);
        let bdt: Time<Beidou> = gal.into_scale().unwrap();
        let back: Time<Galileo> = bdt.into_scale().unwrap();

        assert_eq!(gal, back);
    }

    #[test]
    fn test_glonass_epoch_to_utc_nanos() {
        let glo = Time::<Glonass>::EPOCH;
        let utc: Time<Utc> = glo.into_scale().unwrap();

        // GLONASS epoch = 1995-12-31 21:00:00 UTC = 757_371_600 seconds from 1972
        assert_eq!(utc.as_nanos(), 757_371_600_000_000_000);
    }

    #[test]
    fn test_utc_at_glonass_epoch_gives_zero() {
        let utc = Time::<Utc>::from_nanos(757_371_600_000_000_000);
        let glo: Time<Glonass> = utc.into_scale().unwrap();

        assert_eq!(glo, Time::<Glonass>::EPOCH);
    }

    #[test]
    fn test_glonass_utc_glonass_roundtrip() {
        let glo = Time::<Glonass>::from_day_tod(
            10_000,
            DurationParts {
                seconds: 36_000,
                nanos: 0,
            },
        )
        .unwrap();
        let utc: Time<Utc> = glo.into_scale().unwrap();
        let back: Time<Glonass> = utc.into_scale().unwrap();

        assert_eq!(glo, back);
    }

    #[test]
    fn test_utc_before_glonass_epoch_is_error() {
        let utc = Time::<Utc>::EPOCH;
        let result: Result<Time<Glonass>, _> = utc.into_scale();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_gps_utc_gps_roundtrip_at_gps_epoch() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::EPOCH;
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_utc_gps_roundtrip_at_2020() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_utc_roundtrip_exact_at_nanosecond_level() {
        let ls = LeapSeconds::builtin();
        // Use a timestamp with a non-zero nanosecond component
        let gps = Time::<Gps>::from_nanos(1_167_264_100_123_456_789);
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

        assert_eq!(gps, back); // exact, no rounding
    }

    #[test]
    fn test_gps_leads_utc_by_18s_at_2017_01_01() {
        let ls = LeapSeconds::builtin();
        // 2017-01-01 UTC: 16,437 days * 86,400 s from 1972-01-01
        let expected_utc_s: u64 = 16_437 * 86_400;
        // GPS seconds for this UTC moment:
        // GPS = UTC - epoch_offset + (n - 19)
        // where n = 37, epoch_offset = 252,892,800 s
        let gps_s: u64 = 1_167_264_000 + 18; // pre-verified
        let gps = Time::<Gps>::from_seconds(gps_s);
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();

        assert_eq!(utc.as_seconds(), expected_utc_s);
    }

    #[test]
    fn test_gps_leads_utc_by_13s_at_1999_01_01() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::from_seconds(599_184_013);
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let expected_utc_s: u64 = 9_862 * 86_400; // days from 1972 to 1999

        assert_eq!(utc.as_seconds(), expected_utc_s);
    }

    #[test]
    fn test_gps_glonass_gps_roundtrip() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::from_week_tow(
            2100,
            DurationParts {
                seconds: 86_400,
                nanos: 0,
            },
        )
        .unwrap();
        let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_normal_gps_gives_exact_convert_result() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::from_week_tow(
            2086,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();
        let result: ConvertResult<Time<Utc>> = gps.into_scale_with_checked(ls).unwrap();

        assert!(result.is_exact());
    }

    #[test]
    fn test_utc_to_gps_always_exact() {
        let ls = LeapSeconds::builtin();
        let utc = Time::<Utc>::from_nanos(757_371_600_000_000_000 + 1_000_000_000);
        let result: ConvertResult<Time<Gps>> = utc.into_scale_with_checked(ls).unwrap();

        assert!(result.is_exact());
    }

    #[test]
    fn test_into_inner_returns_value() {
        let t = Time::<Gps>::from_seconds(100);
        let r = ConvertResult::Exact(t);

        assert_eq!(r.into_inner(), t);

        let t2 = Time::<Gps>::from_seconds(200);
        let r2 = ConvertResult::AmbiguousLeapSecond(t2);

        assert_eq!(r2.into_inner(), t2);
    }

    #[test]
    fn test_gps_to_tai_overflow_at_max() {
        let gps = Time::<Gps>::MAX;
        let result: Result<Time<Tai>, _> = gps.into_scale();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_into_scale_gps_tai_matches_to_tai() {
        let gps = Time::<Gps>::from_seconds(999_999);
        let via_trait: Time<Tai> = gps.into_scale().unwrap();
        let via_method = gps.to_tai().unwrap();

        assert_eq!(via_trait, via_method);
    }

    #[test]
    fn test_into_scale_with_gps_utc_matches_gps_to_utc() {
        use crate::leap::{gps_to_utc, LeapSeconds};
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::from_seconds(599_184_013);
        let via_trait: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let via_fn = gps_to_utc(gps, ls).unwrap();

        assert_eq!(via_trait, via_fn);
    }

    #[test]
    fn test_gps_to_utc_detects_leap_second_ambiguity() {
        let ls = LeapSeconds::builtin();
        // GPS time прямо на leap second boundary (2017-01-01)
        let gps = Time::<Gps>::from_seconds(1_167_264_018);
        let result: ConvertResult<Time<Utc>> = gps.into_scale_with_checked(ls).unwrap();

        assert!(matches!(result, ConvertResult::AmbiguousLeapSecond(_)));
    }

    #[test]
    fn test_all_roundtrip_invariants() {
        let ls = LeapSeconds::builtin();

        let gps_values = [
            Time::<Gps>::from_week_tow(
                2086,
                DurationParts {
                    seconds: 0,
                    nanos: 0,
                },
            )
            .unwrap(),
            Time::<Gps>::from_week_tow(
                2100,
                DurationParts {
                    seconds: 86_400,
                    nanos: 0,
                },
            )
            .unwrap(),
            Time::<Gps>::from_nanos(1_167_264_100_123_456_789),
        ];

        for gps in gps_values {
            let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
            let back: Time<Gps> = utc.into_scale_with(ls).unwrap();
            assert_eq!(gps, back);

            let gal: Time<Galileo> = gps.into_scale().unwrap();
            let back: Time<Gps> = gal.into_scale().unwrap();
            assert_eq!(gps, back);

            let bdt: Time<Beidou> = gps.into_scale().unwrap();
            let back: Time<Gps> = bdt.into_scale().unwrap();
            assert_eq!(gps, back);
        }
    }

    #[test]
    fn test_gps_epoch_to_utc_is_exact() {
        let ls = LeapSeconds::builtin();

        let gps = Time::<Gps>::EPOCH;
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();

        assert_eq!(utc.as_seconds(), 252_892_800);
    }

    #[test]
    fn test_gps_epoch_utc_roundtrip() {
        let ls = LeapSeconds::builtin();

        let gps = Time::<Gps>::EPOCH;
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_glonass_roundtrip_invariants_supported_range() {
        let ls = LeapSeconds::builtin();

        let gps_values = [
            Time::<Gps>::from_week_tow(
                2086,
                DurationParts {
                    seconds: 0,
                    nanos: 0,
                },
            )
            .unwrap(),
            Time::<Gps>::from_week_tow(
                2100,
                DurationParts {
                    seconds: 86_400,
                    nanos: 0,
                },
            )
            .unwrap(),
            Time::<Gps>::from_nanos(1_167_264_100_123_456_789),
        ];

        for gps in gps_values {
            let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
            let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

            assert_eq!(gps, back);
        }
    }

    #[test]
    fn test_checked_variants_contract() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::from_week_tow(
            2000,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();
        let res: ConvertResult<Time<Utc>> = gps.into_scale_with_checked(ls).unwrap();

        match res {
            ConvertResult::Exact(_) => {}
            ConvertResult::AmbiguousLeapSecond(_) => panic!("unexpected ambiguity"),
        }
    }

    #[test]
    fn test_convert_result_consistency() {
        let t = Time::<Gps>::from_seconds(42);
        let exact = ConvertResult::Exact(t);

        assert!(exact.is_exact());
        assert!(!exact.is_ambiguous());

        let amb = ConvertResult::AmbiguousLeapSecond(t);

        assert!(!amb.is_exact());
        assert!(amb.is_ambiguous());
    }

    #[test]
    fn test_gps_to_tai_overflow_near_max() {
        let gps = Time::<Gps>::from_nanos(Time::<Gps>::MAX.as_nanos() - 1);
        let result: Result<Time<Tai>, _> = gps.into_scale();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_gps_to_tai_near_overflow_succeeds() {
        let gps = Time::<Gps>::from_nanos(Time::<Gps>::MAX.as_nanos() - 20_000_000_000);
        let tai: Time<Tai> = gps.into_scale().unwrap();

        assert!(tai.as_nanos() > gps.as_nanos());
    }

    #[test]
    fn test_glonass_utc_symmetry_random() {
        let utc = Time::<Utc>::from_nanos(800_000_000_000_000_000);
        let glo: Time<Glonass> = utc.into_scale().unwrap();
        let back: Time<Utc> = glo.into_scale().unwrap();

        assert_eq!(utc, back);
    }
}
