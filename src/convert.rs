//! # Unified type-safe conversion API
//!
//! This module provides the [`IntoScale`] and [`IntoScaleWith`] traits for
//! converting between time scales, plus [`ConvertResult`] for handling the
//! one-second ambiguous window that occurs during leap second insertion.
//!
//! ## Quick overview
//!
//! | From → To        | API                                | Context?            |
//! |------------------|------------------------------------|---------------------|
//! | `GPS → TAI`      | [`IntoScale::into_scale`]          | no                  |
//! | `GPS → Galileo`  | [`IntoScale::into_scale`]          | no                  |
//! | `GPS → BeiDou`   | [`IntoScale::into_scale`]          | no                  |
//! | `Galileo → GPS`  | [`IntoScale::into_scale`]          | no                  |
//! | `BeiDou → GPS`   | [`IntoScale::into_scale`]          | no                  |
//! | `GPS → UTC`      | [`IntoScaleWith::into_scale_with`] | LeapSecondsProvider |
//! | `UTC → GPS`      | [`IntoScaleWith::into_scale_with`] | LeapSecondsProvider |
//! | `GLO → UTC`      | [`IntoScale::into_scale`]          | no (constant shift) |
//! | `UTC → GLO`      | [`IntoScale::into_scale`]          | no (constant shift) |
//! | `GPS → GLO`      | [`IntoScaleWith::into_scale_with`] | LeapSecondsProvider |
//! | `GLO → GPS`      | [`IntoScaleWith::into_scale_with`] | LeapSecondsProvider |
//!
//! ## Ergonomic usage
//!
//! ```rust
//! use gnss_time::{Galileo, Gps, IntoScale, IntoScaleWith, LeapSeconds, Tai, Time, Utc};
//!
//! // Fixed-offset conversions — no context needed
//! let gps = Time::<Gps>::from_week_tow(2345, 0.0).unwrap();
//! let tai: Time<Tai> = gps.into_scale().unwrap();
//! let gal: Time<Galileo> = gps.into_scale().unwrap();
//!
//! // Contextual conversions — explicit leap seconds
//! let ls = LeapSeconds::builtin();
//! let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
//! ```
//!
//! ## Leap second ambiguity
//!
//! When converting `GPS -> UTC`, the 1-second window of ;eap second insertion
//! produces a UTC value that is technically ambiguous: the same UTC nanosecond
//! count could represent either the **inserted** leap second (`23:59:60`) or
//! the first second of the new minute (`00:00:00`).
//!
//! Use [`IntoScaleWith::into_scale_with_checked`] to detect this:
//!
//! ```rust
//! use gnss_time::{ConvertResult, Gps, IntoScaleWith, LeapSeconds, Time, Utc};
//!
//! let ls = LeapSeconds::builtin();
//!
//! // 2017-01-01 00:00:00 GPS (18 s ahead of UTC after the leap)
//! let gps = Time::<Gps>::from_seconds(1_167_264_018);
//! let result: Result<ConvertResult<Time<Utc>>, _> = gps.into_scale_with_checked(ls);
//!
//! assert!(matches!(result, Ok(ConvertResult::AmbiguousLeapSecond(_))));
//! ```

use crate::{
    glonass_to_gps, glonass_to_utc, gps_to_glonass, gps_to_utc, utc_to_glonass, utc_to_gps, Beidou,
    Galileo, Glonass, GnssTimeError, Gps, LeapSecondsProvider, Tai, Time, TimeScale, Utc,
};

/// Convert a `Time<Self>` into `Time<Target>` using a fixed offset.
///
/// Only available for scale pairs that have a **compile-time constant** offset
/// (GLONASS..UTC, GPS..TAI, GPS..Galileo, GPS..BeiDou, Galileo..BeiDou).
///
/// # Errors
///
/// [`GnssTimeError::Overflow`] if the result lies outside `[0, u64::MAX]` ns.
pub trait IntoScale<Target: TimeScale>: Sized {
    /// Perform the fixed-offset conversion.
    fn into_scale(self) -> Result<Time<Target>, GnssTimeError>;
}

/// Convert a `Time<Self>` into `Time<Target>` using a leap-second provider.
///
/// Required for conversions that cross the UTC <-> TAI boundary, where the
/// number of accumulated leap seconds changes over time.
///
/// # Leap second ambiguty
///
/// Use [`into_scale_with_checked`](Self::into_scale_with_checked) to detect
/// the 1-second ambiguous window during leap second insertion.
pub trait IntoScaleWith<Target: TimeScale>: Sized {
    /// Performanceyje contextual conversion, returning `Err` on overflow.
    ///
    /// During leap second insertion the result is still returned as `Ok`,
    /// but may be subtly off by 1s for the duration of the leap second.
    /// Use [`into_scale_with_checked`](Self::into_scale_with_checked) if you
    /// need to detect this.
    fn into_scale_with<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<Time<Target>, GnssTimeError>;

    /// Like [`into_scale_with`](Self::into_scale_with), but additionally
    /// signals when the result falls inside a leap second window.
    ///
    /// Returns [`ConvertResult::AmbiguousLeapSecond`] when the GPS timestamp
    /// corresponds to the inserted leap second second (the "61st second").
    fn into_scale_with_checked<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<ConvertResult<Time<Target>>, GnssTimeError>;
}

/// Result of a conversion that may be ambiguous during leap second insertion.
///
/// During the 1-second leap second window, a GPS timestamp maps to a UTC value
/// that lies within the "64st second" of the minute. Most wall_clock systems
/// cannot represent `23:59:60`, so we report the next representable value
/// together with a flag indicating the situation.
///
/// For all other moments the result is [`Exact`](ConvertResult::Exact).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertResult<T> {
    /// Unambiguous conversion - the only correct value.
    Exact(T),

    /// The source timestamp falls inside the 1-second leap second window.
    ///
    /// The inner value is the first nanosecond of the *new* minute in UTC.
    /// The actual leap second occupies the interval
    /// `(inner - 1_000_000_000 ns, inner)`.
    AmbiguousLeapSecond(T),
}

impl<T> ConvertResult<T> {
    /// Unwrap the inner value regardless of the variant.
    #[inline]
    pub fn into_inner(self) -> T {
        match self {
            ConvertResult::Exact(t) | ConvertResult::AmbiguousLeapSecond(t) => t,
        }
    }

    /// Returns `true` if this is an unambiguous result.
    #[inline]
    pub fn is_exact(&self) -> bool {
        matches!(self, ConvertResult::Exact(_))
    }

    /// Returns `true` if this falls inside a leap second insertion window.
    #[inline]
    pub fn is_unambiguous(&self) -> bool {
        matches!(self, ConvertResult::AmbiguousLeapSecond(_))
    }
}

impl IntoScale<Utc> for Time<Glonass> {
    /// GLONASS -> UTC: constant epoch shift (+757 371 600 s, no leap seconds).
    ///
    /// GLONASS tracks UTC(SU) = UTC + 3 h, including leap second insertions.
    /// Converting to/from UTC is therefore a simple constant epoch shift.
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::IntoScale,
    ///     scale::{Glonass, Utc},
    ///     Time,
    /// };
    ///
    /// let glo = Time::<Glonass>::from_day_tod(0, 0.0).unwrap(); // GLO epoch
    /// let utc: Time<Utc> = glo.into_scale().unwrap();
    ///
    /// // UTC at GLO epoch: 1995-12-31 21:00:00 UTC = 757_371_600 s from 1972
    /// assert_eq!(utc.as_nanos(), 757_371_600_000_000_000);
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Utc>, GnssTimeError> {
        glonass_to_utc(self)
    }
}

impl IntoScale<Glonass> for Time<Utc> {
    /// UTC -> GLONASS: constant epoch shift.
    ///
    /// # Errors
    ///
    /// [`GnssTimeError::Overflow`] if UTC is before the GLONASS epoch
    /// (1995-12-31 21:00:00 UTC).
    #[inline]
    fn into_scale(self) -> Result<Time<Glonass>, GnssTimeError> {
        utc_to_glonass(self)
    }
}

impl IntoScale<Tai> for Time<Gps> {
    /// GPS -> TAI: add 19 seconds (constant, no leap seconds).
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::IntoScale,
    ///     scale::{Gps, Tai},
    ///     Time,
    /// };
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

impl IntoScale<Gps> for Time<Tai> {
    /// TAI -> GPS: substract 19 seconds.
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::IntoScale,
    ///     scale::{Gps, Tai},
    ///     Time,
    /// };
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

impl IntoScale<Galileo> for Time<Gps> {
    /// GPS -> Galoleo: identify on nanoseconds (both share `TAI - 19s`).
    ///
    /// A GPS and a Galileo timestamp with the same nanosecond cout represent
    /// the **same physical instant**.
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::IntoScale,
    ///     scale::{Galileo, Gps},
    ///     Time,
    /// };
    ///
    /// let gps = Time::<Gps>::from_seconds(12_345);
    /// let gal: Time<Galileo> = gps.into_scale().unwrap();
    ///
    /// assert_eq!(gps.as_nanos(), gal.as_nanos());
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Galileo>, GnssTimeError> {
        // GPS and Galileo share the same TAI offset (19s) -> round-trip via TAI is an
        // identify on nanoseconds.
        self.try_convert::<Galileo>()
    }
}

impl IntoScale<Gps> for Time<Galileo> {
    /// Galileo -> GPS: identify on nanoseconds.
    #[inline]
    fn into_scale(self) -> Result<Time<Gps>, GnssTimeError> {
        self.try_convert::<Gps>()
    }
}

impl IntoScale<Beidou> for Time<Gps> {
    /// GPS -> BeiDou: `BDT = GPS - 14s` (via TAI: GPS + 19s TAI, BDT + 33s
    /// TAI).
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::IntoScale,
    ///     scale::{Beidou, Gps},
    ///     Time,
    /// };
    ///
    /// let gps = Time::<Gps>::from_seconds(100);
    /// let bdt: Time<Beidou> = gps.into_scale().unwrap();
    ///
    /// assert_eq!(bdt.as_seconds(), 86); // 100 + 19 - 33 = 86
    /// ```
    #[inline]
    fn into_scale(self) -> Result<Time<Beidou>, GnssTimeError> {
        self.try_convert::<Beidou>()
    }
}

impl IntoScale<Gps> for Time<Beidou> {
    /// BeiDou -> GPS: `GPS = BDT + 14s`.
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::IntoScale,
    ///     scale::{Beidou, Gps},
    ///     Time,
    /// };
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

impl IntoScale<Beidou> for Time<Galileo> {
    /// Galileo -> BeiDou via TAI.
    #[inline]
    fn into_scale(self) -> Result<Time<Beidou>, GnssTimeError> {
        self.try_convert::<Beidou>()
    }
}

impl IntoScale<Galileo> for Time<Beidou> {
    /// BeiDou -> Galileo via TAI.
    #[inline]
    fn into_scale(self) -> Result<Time<Galileo>, GnssTimeError> {
        self.try_convert::<Galileo>()
    }
}

impl IntoScaleWith<Utc> for Time<Gps> {
    /// GPS -> UTC with leap second context.
    ///
    /// Roundtrip accuracy: `GPS -> UTC -> GPS` is exact (< 1ns error) for all
    /// times iutside the 1-second leap second insertion window.
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::IntoScaleWith,
    ///     leap::LeapSeconds,
    ///     scale::{Gps, Utc},
    ///     Time,
    /// };
    ///
    /// let ls = LeapSeconds::builtin();
    /// let gps = Time::<Gps>::from_seconds(1_167_264_018); // 2017-01-01 GPS
    /// let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
    ///
    /// let delta = gps.as_seconds() as i64 - utc.as_seconds() as i64 + 252_892_800_i64;
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

        // Detect leap second window: compute the TAI at this GPS moment and compare the
        // leap count just before and just after. If they differ, we are inside (or just
        // crossed) a leap second boundary.
        let tai = self.to_tai()?;
        let n_at = ls.tai_minus_utc_at(tai);

        // Check 1s earlier to detect antry into the leap second
        let tai_prev = if tai.as_nanos() >= 1_000_000_000 {
            Time::<Tai>::from_nanos(tai.as_nanos() - 1_000_000_000)
        } else {
            tai
        };
        let n_before = ls.tai_minus_utc_at(tai_prev);

        if n_at != n_before {
            // We crossed a leap second boundary within the last second.
            // The GPS second corresponding to n_before (old count) is the "ambiguous" one.
            Ok(ConvertResult::AmbiguousLeapSecond(utc))
        } else {
            Ok(ConvertResult::Exact(utc))
        }
    }
}

impl IntoScaleWith<Gps> for Time<Utc> {
    /// UTC -> GPS with leap second context.
    ///
    /// ```rust
    /// use gnss_time::{
    ///     convert::{IntoScale, IntoScaleWith},
    ///     leap::LeapSeconds,
    ///     scale::{Gps, Utc},
    ///     Time,
    /// };
    ///
    /// let ls = LeapSeconds::builtin();
    /// let gps_orig = Time::<Gps>::from_week_tow(2086, 0.0).unwrap();
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
        // UTC -> GPS is unambiguous: each UTC nanosecond maps to exactly one GPS
        // nanosecond (GPS has no gaps or repeated seconds).
        Ok(ConvertResult::Exact(utc_to_gps(self, &ls)?))
    }
}

impl IntoScaleWith<Glonass> for Time<Gps> {
    /// GPS -> GLONASS via UTC.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LeapSeconds;

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
        let gps = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
        let tai: Time<Tai> = gps.into_scale().unwrap();
        let back: Time<Gps> = tai.into_scale().unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_tai_to_gps_underflow_at_tai_zero() {
        // TAI(0) - 19 s → negative GPS → overflow
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
        let gps = Time::<Gps>::from_week_tow(2000, 123_456.789).unwrap();
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
        let gps = Time::<Gps>::from_week_tow(2100, 86_400.0).unwrap();
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

        // GLONASS epoch = 1995-12-31 21:00:00 UTC = 757_371_600 s from 1972
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
        let glo = Time::<Glonass>::from_day_tod(10_000, 36_000.0).unwrap();
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
        let gps = Time::<Gps>::from_week_tow(2086, 0.0).unwrap();
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

        assert_eq!(gps, back);
    }

    #[test]
    fn test_gps_utc_roundtrip_exact_at_nanosecond_level() {
        let ls = LeapSeconds::builtin();
        // Use a timestamp with non-zero nanosecond component
        let gps = Time::<Gps>::from_nanos(1_167_264_100_123_456_789);
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = utc.into_scale_with(ls).unwrap();

        assert_eq!(gps, back); // exact, no rounding
    }

    /// Verify GPS-UTC = 18 s on 2017-01-01 00:00:00 UTC.
    #[test]
    fn test_gps_leads_utc_by_18s_at_2017_01_01() {
        let ls = LeapSeconds::builtin();
        // 2017-01-01 UTC: 16437 days * 86400 s from 1972-01-01
        let expected_utc_s: u64 = 16_437 * 86_400;
        // GPS seconds for that UTC moment: GPS = UTC - epoch_offset + (n - 19)
        // where n=37, epoch_offset = 252_892_800 s
        let gps_s: u64 = 1_167_264_000 + 18; // pre-verified
        let gps = Time::<Gps>::from_seconds(gps_s);
        let utc: Time<Utc> = gps.into_scale_with(ls).unwrap();

        assert_eq!(utc.as_seconds(), expected_utc_s);
    }

    /// Verify GPS-UTC = 13 s on 1999-01-01 00:00:00 UTC.
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
        let gps = Time::<Gps>::from_week_tow(2100, 86_400.0).unwrap();
        let glo: Time<Glonass> = gps.into_scale_with(ls).unwrap();
        let back: Time<Gps> = glo.into_scale_with(ls).unwrap();

        assert_eq!(gps, back);
    }

    /// Normal time → Exact result.
    #[test]
    fn test_normal_gps_gives_exact_convert_result() {
        let ls = LeapSeconds::builtin();
        let gps = Time::<Gps>::from_week_tow(2086, 0.0).unwrap();
        let result: ConvertResult<Time<Utc>> = gps.into_scale_with_checked(ls).unwrap();

        assert!(result.is_exact());
    }

    /// UTC → GPS always produces Exact (GPS has no ambiguous seconds).
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
}
