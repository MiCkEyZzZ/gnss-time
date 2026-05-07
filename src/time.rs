//! # `Time<S>` — the core timestamp type.
//!
//! Stores **nanoseconds since the epoch of scale `S`** in a `u64`.
//! The phantom `S: TimeScale` enforces domain correctness at compile time —
//! you cannot subtract a GPS timestamp from a GLONASS timestamp.
//!
//! ## Size guarantee
//!
//! ```rust
//! # use gnss_time::{Time, scale::Gps};
//!
//! assert_eq!(core::mem::size_of::<Time<Gps>>(), 8); // identical to u64
//! ```
//!
//! ## Representable range and overflow semantics
//!
//! The internal representation is a `u64` counting **nanoseconds from the
//! scale's epoch**.  `u64::MAX` nanoseconds ≈ **584.5 years**, so:
//!
//! | Scale   | Epoch            | `Time::MAX` corresponds to |
//! |---------|------------------|----------------------------|
//! | GLONASS | 1996-01-01       | ≈ **2580-07-01**           |
//! | GPS     | 1980-01-06       | ≈ **2564-07-04**           |
//! | Galileo | 1999-08-22       | ≈ **2584-02-15**           |
//! | BeiDou  | 2006-01-01       | ≈ **2590-07-02**           |
//! | TAI     | 1958-01-01       | ≈ **2542-07-05**           |
//! | UTC     | 1972-01-01       | ≈ **2556-07-03**           |
//!
//! ## Unix time interoperability
//!
//! `Time<Utc>` counts nanoseconds from **1972-01-01** (UTC epoch), while Unix
//! time starts from **1970-01-01**. The gap is [`UTC_EPOCH_UNIX_OFFSET_S`] =
//! 63 072 000 s (730 days).
//!
//! ```rust
//! use gnss_time::{Time, Utc};
//!
//! // Unix epoch (1970-01-01) is before the UTC epoch (1972-01-01) → error
//! assert!(Time::<Utc>::from_unix_seconds(0).is_err());
//!
//! // 1972-01-01 = UTC epoch
//! let utc = Time::<Utc>::from_unix_seconds(63_072_000).unwrap();
//! assert_eq!(utc, Time::<Utc>::EPOCH);
//! assert_eq!(utc.as_unix_seconds(), 63_072_000);
//! ```
//!
//! ## Arithmetic overflow semantics
//!
//! All arithmetic is **checked by default** - panicking operators (`+`, `-`)
//! are only suitable for cases you know cannot oberflow. Fo embedded code or
//! long-running servers, prefer:
//!
//! ```rust
//! use gnss_time::{scale::Gps, Duration, Time};
//!
//! let t = Time::<Gps>::MAX;
//! let d = Duration::from_seconds(1);
//!
//! // Checked - returns None on overflow
//! assert!(t.checked_add(d).is_none());
//!
//! // Saturating - clamps at MAX/EPOCH instead of panicking
//! assert_eq!(t.saturating_add(d), Time::<Gps>::MAX);
//!
//! // Fallible - returns Err(GnssTimeError::Overflow)
//! assert!(t.try_add(d).is_err());
//! ```

use core::{
    fmt,
    marker::PhantomData,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use crate::{
    gps_to_utc, utc_to_gps, DisplayStyle, Duration, Glonass, GnssTimeError, Gps, LeapSeconds,
    LeapSecondsProvider, OffsetToTai, Tai, TimeScale, Utc, UTC_EPOCH_UNIX_OFFSET_NS,
    UTC_EPOCH_UNIX_OFFSET_S,
};

/// A timestamp in time scale `S`, stored as nanoseconds since the epoch of the
/// scale.
///
/// # Examples
///
/// ```rust
/// use gnss_time::{Duration, Glonass, Gps, Time};
///
/// let t: Time<Gps> = Time::from_nanos(0); // GPS epoch
/// let later = t + Duration::from_seconds(3600);
///
/// assert_eq!((later - t).as_seconds(), 3600);
///
/// // Compile-time error — different time scales are incompatible:
/// // let glo: Time<Glonass> = Time::from_nanos(0);
/// // let _ = later - glo; // ← ERROR
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[must_use = "Time<S> is a value type; ignoring it has no effect"]
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,
}

/// Split seconds into whole seconds and nanoseconds.
///
/// This type is used for GNSS week/day constructors so that the core API
/// stays fully deterministic and `no_std`-friendly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DurationParts {
    /// Whole seconds part (non-negative).
    pub seconds: u64,

    /// Nanosecond part, must be in `[0, 999_999_999]`.
    pub nanos: u32,
}

impl<S: TimeScale> Time<S> {
    /// The scale's epoch — 0 nanoseconds.
    ///
    /// Corresponds to the calendar date defined by [`TimeScale::EPOCH_CIVIL`]
    /// (e.g. `1980-01-06` for GPS, `1996-01-01` for GLONASS).
    pub const EPOCH: Self = Time {
        nanos: 0,
        _scale: PhantomData,
    };

    /// Minimum representable value (synonym for `EPOCH`).
    pub const MIN: Self = Self::EPOCH;

    /// Maximum representable instant (`u64::MAX` nanoseconds ≈ 584.5 years).
    pub const MAX: Self = Time {
        nanos: u64::MAX,
        _scale: PhantomData,
    };

    /// Nanoseconds per non-leap year (365 days).
    ///
    /// Useful for sanity-checking that a value is within a reasonable range:
    /// ```rust
    /// use gnss_time::{scale::Gps, Time};
    ///
    /// // 50 years from GPS epoch
    /// let fifty_years = Time::<Gps>::from_nanos(50 * Time::<Gps>::NANOS_PER_YEAR);
    /// assert!(fifty_years.as_nanos() > 0);
    /// ```
    pub const NANOS_PER_YEAR: u64 = 365 * 24 * 3_600 * 1_000_000_000;

    /// Construct from raw nanoseconds since this scale's epoch.
    #[inline(always)]
    pub const fn from_nanos(nanos: u64) -> Self {
        Time {
            nanos,
            _scale: PhantomData,
        }
    }

    /// Construct from whole seconds since this scale's epoch.
    #[inline]
    pub const fn from_seconds(secs: u64) -> Self {
        match secs.checked_mul(1_000_000_000) {
            Some(n) => Time::from_nanos(n),
            None => panic!("Time::from_seconds overflow"),
        }
    }

    /// Construct from whole seconds, returning `None` on overflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_seconds(secs: u64) -> Option<Self> {
        match secs.checked_mul(1_000_000_000) {
            Some(n) => Some(Time::from_nanos(n)),
            None => None,
        }
    }

    /// Raw nanoseconds since this scale's epoch.
    #[inline(always)]
    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    /// Whole seconds since this scale's epoch (truncated).
    #[inline]
    #[must_use]
    pub const fn as_seconds(self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Seconds as `f64`. For large timestamps (> ~2^53 ns), precision loss
    /// affects even milliseconds
    #[inline]
    #[must_use]
    pub fn as_seconds_f64(self) -> f64 {
        self.nanos as f64 / 1_000_000_000.0
    }

    /// Convert to TAI using the scale's fixed offset.
    ///
    /// Returns [`GnssTimeError::LeapSecondsRequired`] for contextual scales
    /// (UTC, GLONASS) and [`GnssTimeError::Overflow`] for out-of-range results.
    pub fn to_tai(self) -> Result<Time<Tai>, GnssTimeError> {
        match S::OFFSET_TO_TAI {
            OffsetToTai::Fixed(offset) => {
                let nanos = (self.nanos as i128) + (offset as i128);

                if nanos < 0 || nanos > u64::MAX as i128 {
                    return Err(GnssTimeError::Overflow);
                }

                Ok(Time::from_nanos(nanos as u64))
            }
            OffsetToTai::Contextual => Err(GnssTimeError::LeapSecondsRequired),
        }
    }

    /// Construct `Time<S>` from a TAI timestamp using the scale's fixed offset.
    pub fn from_tai(tai: Time<Tai>) -> Result<Self, GnssTimeError> {
        match S::OFFSET_TO_TAI {
            OffsetToTai::Fixed(offset) => {
                let nanos = (tai.as_nanos() as i128) - (offset as i128);

                if nanos < 0 || nanos > u64::MAX as i128 {
                    return Err(GnssTimeError::Overflow);
                }

                Ok(Time::from_nanos(nanos as u64))
            }
            OffsetToTai::Contextual => Err(GnssTimeError::LeapSecondsRequired),
        }
    }

    /// Convert directly between two fixed-offset scales via TAI.
    ///
    /// Fails if either source or target scale requires leap seconds.
    pub fn try_convert<T: TimeScale>(self) -> Result<Time<T>, GnssTimeError> {
        let tai = self.to_tai()?;

        Time::<T>::from_tai(tai)
    }

    /// Add a `Duration`, returning `None` on overflow or underflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub fn checked_add(
        self,
        d: Duration,
    ) -> Option<Self> {
        let result = (self.nanos as i128) + (d.as_nanos() as i128);

        if result < 0 || result > u64::MAX as i128 {
            return None;
        }

        Some(Time::from_nanos(result as u64))
    }

    /// Subtract a `Duration`, returning `None` on overflow or underflow.
    #[inline]
    #[must_use = "returns None on underflow; check the result"]
    pub fn checked_sub_duration(
        self,
        d: Duration,
    ) -> Option<Self> {
        let result = (self.nanos as i128) - (d.as_nanos() as i128);

        if result < 0 || result > u64::MAX as i128 {
            return None;
        }

        Some(Time::from_nanos(result as u64))
    }

    /// Add, saturating at `EPOCH` (below) and `MAX` (above).
    #[inline]
    #[must_use = "saturating_add returns a new Time<S>; the original is unchanged"]
    pub fn saturating_add(
        self,
        d: Duration,
    ) -> Self {
        self.checked_add(d).unwrap_or(if d.is_negative() {
            Time::EPOCH
        } else {
            Time::MAX
        })
    }

    /// Subtract duration, saturating at bounds.
    #[inline]
    #[must_use = "saturating_sub_duration returns a new Time<S>; the original is unchanged"]
    pub fn saturating_sub_duration(
        self,
        d: Duration,
    ) -> Self {
        self.checked_sub_duration(d).unwrap_or(if d.is_negative() {
            Time::MAX
        } else {
            Time::EPOCH
        })
    }

    /// Fallible add — [`GnssTimeError::Overflow`] on failure.
    #[inline]
    pub fn try_add(
        self,
        d: Duration,
    ) -> Result<Self, GnssTimeError> {
        self.checked_add(d).ok_or(GnssTimeError::Overflow)
    }

    /// Fallible subtract — [`GnssTimeError::Overflow`] on failure.
    #[inline]
    pub fn try_sub_duration(
        self,
        d: Duration,
    ) -> Result<Self, GnssTimeError> {
        self.checked_sub_duration(d).ok_or(GnssTimeError::Overflow)
    }

    /// Signed interval `self − earlier`. Returns `None` if it overflows `i64`.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_elapsed(
        self,
        earlier: Time<S>,
    ) -> Option<Duration> {
        let diff = (self.nanos as i128) - (earlier.nanos as i128);

        if diff > i64::MAX as i128 || diff < i64::MIN as i128 {
            return None;
        }

        Some(Duration::from_nanos(diff as i64))
    }
}

impl<S: TimeScale> Add<Duration> for Time<S> {
    type Output = Time<S>;

    #[inline]
    fn add(
        self,
        rhs: Duration,
    ) -> Time<S> {
        self.checked_add(rhs)
            .expect("Time<S> + Duration overflowed")
    }
}

impl<S: TimeScale> AddAssign<Duration> for Time<S> {
    #[inline]
    fn add_assign(
        &mut self,
        rhs: Duration,
    ) {
        *self = *self + rhs
    }
}

impl<S: TimeScale> Sub<Duration> for Time<S> {
    type Output = Time<S>;

    #[inline]
    fn sub(
        self,
        rhs: Duration,
    ) -> Self::Output {
        self.checked_sub_duration(rhs)
            .expect("Time<S> - Duration underflowed")
    }
}

impl<S: TimeScale> SubAssign<Duration> for Time<S> {
    #[inline]
    fn sub_assign(
        &mut self,
        rhs: Duration,
    ) {
        *self = *self - rhs;
    }
}

impl<S: TimeScale> Sub<Time<S>> for Time<S> {
    type Output = Duration;

    #[inline]
    fn sub(
        self,
        rhs: Time<S>,
    ) -> Self::Output {
        self.checked_elapsed(rhs)
            .expect("Time<S> - Time<S> overflowed i64")
    }
}

impl DurationParts {
    /// Number of nanoseconds in one second.
    pub const NANOS_PER_SECOND: u32 = 1_000_000_000;

    /// Creates a new `DurationParts` from whole seconds and nanoseconds.
    ///
    /// # Parameters
    /// - `seconds` – whole seconds (non‑negative)
    /// - `nanos` – additional nanoseconds, **must be less than**
    ///   `1_000_000_000`
    ///
    /// # Errors
    /// Returns [`GnssTimeError::InvalidInput`] if `nanos >= 1_000_000_000`.
    ///
    /// # Example
    /// ```
    /// use gnss_time::DurationParts;
    ///
    /// let parts = DurationParts::new(5, 500_000_000).unwrap();
    ///
    /// assert_eq!(parts.as_nanos(), 5_500_000_000);
    /// ```
    #[inline]
    pub const fn new(
        seconds: u64,
        nanos: u32,
    ) -> Result<Self, GnssTimeError> {
        if nanos >= Self::NANOS_PER_SECOND {
            return Err(GnssTimeError::InvalidInput(
                "nanos must be in [0, 1_000_000_000)",
            ));
        }

        Ok(Self { seconds, nanos })
    }

    /// Converts the `DurationParts` into a total number of nanoseconds as
    /// `u128`.
    ///
    /// # Example
    /// ```
    /// use gnss_time::DurationParts;
    ///
    /// let parts = DurationParts {
    ///     seconds: 2,
    ///     nanos: 123_456_789,
    /// };
    ///
    /// assert_eq!(parts.as_nanos(), 2_123_456_789);
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_nanos(self) -> u128 {
        (self.seconds as u128) * Self::NANOS_PER_SECOND as u128 + self.nanos as u128
    }
}

impl Time<Glonass> {
    /// Construct from GLONASS day number and time-of-day.
    ///
    /// `tod.seconds` must be in `[0, 86_400)`.
    /// `tod.nanos` must be in `[0, 1_000_000_000)`.
    ///
    /// # Errors
    ///
    /// [`GnssTimeError::InvalidInput`] if `tod_s ∉ [0, 86 400)`.
    pub fn from_day_tod(
        day: u32,
        tod: DurationParts,
    ) -> Result<Self, GnssTimeError> {
        if tod.seconds >= 86_400 {
            return Err(GnssTimeError::InvalidInput(
                "tod.seconds must be in [0, 86_400)",
            ));
        }
        if tod.nanos >= DurationParts::NANOS_PER_SECOND {
            return Err(GnssTimeError::InvalidInput(
                "tod.nanos must be in [0, 1_000_000_000)",
            ));
        }

        let day_ns = (day as u64)
            .checked_mul(86_400_000_000_000)
            .ok_or(GnssTimeError::Overflow)?;
        let tod_ns = tod
            .seconds
            .checked_mul(1_000_000_000)
            .ok_or(GnssTimeError::Overflow)?
            .checked_add(tod.nanos as u64)
            .ok_or(GnssTimeError::Overflow)?;
        let total = day_ns.checked_add(tod_ns).ok_or(GnssTimeError::Overflow)?;

        Ok(Time::from_nanos(total))
    }

    /// Day number since GLONASS epoch.
    #[inline]
    #[must_use]
    pub const fn day(self) -> u32 {
        (self.nanos / 86_400_000_000_000u64) as u32
    }

    /// Time of day in whole seconds.
    #[inline]
    #[must_use]
    pub const fn tod_seconds(self) -> u32 {
        ((self.nanos % 86_400_000_000_000u64) / 1_000_000_000u64) as u32
    }

    /// Sub-second nanosecond remainder within the current second.
    #[inline]
    #[must_use]
    pub const fn sub_second_nanos(self) -> u32 {
        (self.nanos % 1_000_000_000u64) as u32
    }

    /// Day of week: **1 = Monday … 7 = Sunday** (NavIC / ISO 8601 convention).
    ///
    /// GLONASS epoch (1996-01-01) was a **Monday**, so day 0 → 1 (Monday).
    ///
    /// The formula is simply `(day % 7) + 1`.
    ///
    /// # GLONASS ICD note
    ///
    /// The GLONASS Interface Control Document defines the "day number within
    /// the four-year interval" (`N_T`) starting from 1, but for simplicity
    /// this crate uses 0-based day counts from the epoch and exposes the
    /// ISO / NavIC weekday (1=Mon … 7=Sun) through this method.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gnss_time::{scale::Glonass, DurationParts, Time};
    ///
    /// // Day 0 = 1996-01-01 = Monday
    /// let t = Time::<Glonass>::from_day_tod(
    ///     0,
    ///     DurationParts {
    ///         seconds: 0,
    ///         nanos: 0,
    ///     },
    /// )
    /// .unwrap();
    ///
    /// assert_eq!(t.day_of_week(), 1); // Monday
    ///
    /// // Day 6 = 1996-01-07 = Sunday
    /// let t2 = Time::<Glonass>::from_day_tod(
    ///     6,
    ///     DurationParts {
    ///         seconds: 0,
    ///         nanos: 0,
    ///     },
    /// )
    /// .unwrap();
    ///
    /// assert_eq!(t2.day_of_week(), 7); // Sunday
    ///
    /// // Day 7 = 1996-01-08 = Monday again
    /// let t3 = Time::<Glonass>::from_day_tod(
    ///     7,
    ///     DurationParts {
    ///         seconds: 0,
    ///         nanos: 0,
    ///     },
    /// )
    /// .unwrap();
    ///
    /// assert_eq!(t3.day_of_week(), 1);
    /// ```
    #[inline]
    #[must_use]
    pub const fn day_of_week(self) -> u8 {
        // GLONASS epoch starts on Monday → day 0 corresponds to 1
        (self.day() % 7) as u8 + 1
    }

    /// Returns `true` if the current day-of-week is Saturday (6) or Sunday (7).
    #[inline]
    #[must_use]
    pub const fn is_weekend(self) -> bool {
        let d = self.day_of_week();

        d == 6 || d == 7
    }
}

impl Time<Gps> {
    /// Construct from GPS week number and time-of-week.
    ///
    /// `tow.seconds` must be in `[0, 604_800)`.
    /// `tow.nanos` must be in `[0, 1_000_000_000)`.
    pub fn from_week_tow(
        week: u16,
        tow: DurationParts,
    ) -> Result<Self, GnssTimeError> {
        if tow.seconds >= 604_800 {
            return Err(GnssTimeError::InvalidInput(
                "tow.seconds must be in [0, 604_800)",
            ));
        }

        if tow.nanos >= DurationParts::NANOS_PER_SECOND {
            return Err(GnssTimeError::InvalidInput(
                "tow.nanos must be in [0, 1_000_000_000)",
            ));
        }

        let week_ns = (week as u64)
            .checked_mul(604_800_000_000_000)
            .ok_or(GnssTimeError::Overflow)?;

        let tow_ns = tow
            .seconds
            .checked_mul(1_000_000_000)
            .ok_or(GnssTimeError::Overflow)?
            .checked_add(tow.nanos as u64)
            .ok_or(GnssTimeError::Overflow)?;

        let total = week_ns.checked_add(tow_ns).ok_or(GnssTimeError::Overflow)?;

        Ok(Time::from_nanos(total))
    }

    /// Создаёт GPS время из Unix timestamp (секунды с 1970-01-01 UTC).
    pub fn from_unix_seconds<P: LeapSecondsProvider>(
        unix_seconds: i64,
        ls: P,
    ) -> Result<Self, GnssTimeError> {
        let utc = Time::<Utc>::from_unix_seconds(unix_seconds)?;

        utc_to_gps(utc, &ls)
    }

    /// Returns this GPS timestamp as a Unix timestamp (whole seconds since
    /// 1970-01-01 UTC).
    ///
    /// The conversion is `GPS -> UTC -> Unix` and therefore requires an
    /// explicit leap-second provider.
    ///
    /// # Errors
    ///
    /// [`GnssTimeError::Overflow`] if the UTC conversion fails.
    ///
    /// ```rust
    /// use gnss_time::{Gps, LeapSeconds, Time};
    ///
    /// let ls = LeapSeconds::builtin();
    /// // GPS epoch = 1980-01-06 → Unix 315_964_800
    /// assert_eq!(Time::<Gps>::EPOCH.as_unix_seconds(ls).unwrap(), 315_964_800);
    /// ```
    pub fn as_unix_seconds<P: LeapSecondsProvider>(
        self,
        ls: P,
    ) -> Result<i64, GnssTimeError> {
        let utc = gps_to_utc(self, &ls)?;

        Ok(utc.as_unix_seconds())
    }

    /// Conversion of GPS time to UTC using the built-in leap seconds table.
    ///
    /// # Accuracy
    ///
    /// For most timestamps, the conversion is accurate to the nanosecond.
    /// During a leap second insertion window (e.g. 2016-12-31 23:59:60 UTC),
    /// the result may differ by up to 1 second. If this is critical, use
    /// [`to_utc_with`](Self::to_utc_with) with a custom provider that properly
    /// handles the ambiguity.
    pub fn to_utc(self) -> Result<Time<Utc>, GnssTimeError> {
        gps_to_utc(self, LeapSeconds::builtin())
    }

    /// Conversion of GPS time to UTC using a custom leap seconds provider.
    ///
    /// The same accuracy notes apply as for [`to_utc`](Self::to_utc):
    /// the conversion is precise for most timestamps, but during a leap second
    /// insertion window it may differ by up to 1 second.
    pub fn to_utc_with<P: LeapSecondsProvider>(
        self,
        ls: &P,
    ) -> Result<Time<Utc>, GnssTimeError> {
        gps_to_utc(self, ls)
    }

    /// GPS week number.
    #[inline]
    #[must_use]
    pub const fn week(self) -> u32 {
        (self.nanos / 604_800_000_000_000u64) as u32
    }

    /// Time of week in whole seconds.
    #[inline]
    #[must_use]
    pub const fn tow_seconds(self) -> u32 {
        ((self.nanos % 604_800_000_000_000u64) / 1_000_000_000u64) as u32
    }

    /// Sub-second nanosecond remainder within the current second.
    #[inline]
    #[must_use]
    pub const fn sub_second_nanos(self) -> u32 {
        (self.nanos % 1_000_000_000u64) as u32
    }
}

impl Time<Utc> {
    /// Construct from a Unix timestamp (whole seconds since 1970-01-01 UTC).
    ///
    /// `Time<Utc>` counts nanoseconds from **1972-01-01** (UTC epoch), while
    /// Unix time starts from **1970-01-01**. The gap is
    /// [`UTC_EPOCH_UNIX_OFFSET_S`] = 63 072 000 s (730 days).
    ///
    /// # Errors
    ///
    /// Returns [`GnssTimeError::Overflow`] if `unix_seconds < 63_072_000`
    /// (i.e. the date is before 1972-01-01 00:00:00 UTC, the UTC epoch).
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::{Time, Utc, UTC_EPOCH_UNIX_OFFSET_S};
    ///
    /// // Unix epoch (1970-01-01) is before the UTC epoch → error
    /// assert!(Time::<Utc>::from_unix_seconds(0).is_err());
    ///
    /// // 63_072_000 s from Unix epoch = 1972-01-01 = UTC epoch
    /// let utc = Time::<Utc>::from_unix_seconds(UTC_EPOCH_UNIX_OFFSET_S).unwrap();
    /// assert_eq!(utc, Time::<Utc>::EPOCH);
    ///
    /// // Round-trip
    /// let unix_s: i64 = 1_700_000_000;
    /// let utc2 = Time::<Utc>::from_unix_seconds(unix_s).unwrap();
    /// assert_eq!(utc2.as_unix_seconds(), unix_s);
    /// ```
    pub fn from_unix_seconds(unix_seconds: i64) -> Result<Self, GnssTimeError> {
        // utc_seconds_from_1972 = unix_seconds − UTC_EPOCH_UNIX_OFFSET_S
        let utc_s = unix_seconds
            .checked_sub(UTC_EPOCH_UNIX_OFFSET_S)
            .ok_or(GnssTimeError::Overflow)?;

        if utc_s < 0 {
            return Err(GnssTimeError::Overflow);
        }

        let nanos = (utc_s as u64)
            .checked_mul(1_000_000_000)
            .ok_or(GnssTimeError::Overflow)?;

        Ok(Time::from_nanos(nanos))
    }

    /// Construct from a Unix timestamp with nanosecond precision.
    ///
    /// `unix_nanos` is the number of nanoseconds since 1970-01-01 00:00:00 UTC.
    ///
    /// # Errors
    ///
    /// Returns [`GnssTimeError::Overflow`] if the result would be before the
    /// UTC epoch (1972-01-01), i.e. `unix_nanos < UTC_EPOCH_UNIX_OFFSET_NS`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::{Time, Utc, UTC_EPOCH_UNIX_OFFSET_NS};
    ///
    /// // UTC epoch in Unix nanoseconds
    /// let utc = Time::<Utc>::from_unix_nanos(UTC_EPOCH_UNIX_OFFSET_NS).unwrap();
    /// assert_eq!(utc, Time::<Utc>::EPOCH);
    ///
    /// // Round-trip
    /// let nanos: i64 = 1_700_000_000_123_456_789;
    /// let utc2 = Time::<Utc>::from_unix_nanos(nanos).unwrap();
    /// assert_eq!(utc2.as_unix_nanos(), nanos);
    /// ```
    pub fn from_unix_nanos(unix_nanos: i64) -> Result<Self, GnssTimeError> {
        // utc_nanos_from_1972 = unix_nanos − UTC_EPOCH_UNIX_OFFSET_NS
        let utc_ns = unix_nanos
            .checked_sub(UTC_EPOCH_UNIX_OFFSET_NS)
            .ok_or(GnssTimeError::Overflow)?;

        if utc_ns < 0 {
            return Err(GnssTimeError::Overflow);
        }

        Ok(Time::from_nanos(utc_ns as u64))
    }

    /// Returns this UTC timestamp as a Unix timestamp (whole seconds since
    /// 1970-01-01 UTC).
    ///
    /// The result is always ≥ [`UTC_EPOCH_UNIX_OFFSET_S`] because `Time<Utc>`
    /// cannot represent dates before 1972-01-01.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::{Time, Utc, UTC_EPOCH_UNIX_OFFSET_S};
    ///
    /// // UTC epoch = 1972-01-01 = Unix 63_072_000
    /// assert_eq!(
    ///     Time::<Utc>::EPOCH.as_unix_seconds(),
    ///     UTC_EPOCH_UNIX_OFFSET_S
    /// );
    ///
    /// // Round-trip
    /// let unix_s: i64 = 1_700_000_000;
    /// let utc = Time::<Utc>::from_unix_seconds(unix_s).unwrap();
    /// assert_eq!(utc.as_unix_seconds(), unix_s);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_unix_seconds(self) -> i64 {
        (self.nanos / 1_000_000_000) as i64 + UTC_EPOCH_UNIX_OFFSET_S
    }

    /// Returns this UTC timestamp as a Unix timestamp with nanosecond
    /// precision (nanoseconds since 1970-01-01 UTC).
    ///
    /// # Overflow note
    ///
    /// `i64` can represent nanoseconds up to ~year 2262 from the Unix epoch.
    /// For timestamps beyond that, this method saturates at `i64::MAX`.
    /// In practice, `Time<Utc>::MAX` corresponds to ~year 2556, which is
    /// beyond `i64` range — plan accordingly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::{Time, Utc, UTC_EPOCH_UNIX_OFFSET_NS};
    ///
    /// assert_eq!(Time::<Utc>::EPOCH.as_unix_nanos(), UTC_EPOCH_UNIX_OFFSET_NS);
    ///
    /// // Round-trip (within i64 range)
    /// let nanos: i64 = 1_700_000_000_123_456_789;
    /// let utc = Time::<Utc>::from_unix_nanos(nanos).unwrap();
    /// assert_eq!(utc.as_unix_nanos(), nanos);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_unix_nanos(self) -> i64 {
        // self.nanos is u64; cast to i64 wraps above i64::MAX (~year 2262).
        // We use saturating_add to avoid UB and stay predictable.
        (self.nanos as i64).saturating_add(UTC_EPOCH_UNIX_OFFSET_NS)
    }

    /// Conversion of UTC time to GPS using the built-in leap seconds table.
    pub fn to_gps(self) -> Result<Time<Gps>, GnssTimeError> {
        utc_to_gps(self, LeapSeconds::builtin())
    }

    /// Conversion of UTC time to GPS using a custom leap seconds provider.
    pub fn to_gps_with<P: LeapSecondsProvider>(
        self,
        ls: &P,
    ) -> Result<Time<Gps>, GnssTimeError> {
        utc_to_gps(self, ls)
    }
}

impl<S: TimeScale> PartialOrd for Time<S> {
    #[inline]
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: TimeScale> Ord for Time<S> {
    #[inline]
    fn cmp(
        &self,
        other: &Self,
    ) -> core::cmp::Ordering {
        self.nanos.cmp(&other.nanos)
    }
}

impl<S: TimeScale> fmt::Debug for Time<S> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Time<{}>({}ns)", S::NAME, self.nanos)
    }
}

impl<S: TimeScale> fmt::Display for Time<S> {
    /// Formatting depends on the [`DisplayStyle`] of the time scale:
    ///
    /// | Style      | Example                    |
    /// |------------|---------------------------|
    /// | `WeekTow`  | `"GPS 2345:432000.000"`   |
    /// | `DayTod`   | `"GLO 10512:43200.000"`   |
    /// | `Simple`   | `"TAI +1000000000s 0ns"`  |
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match S::DISPLAY_STYLE {
            DisplayStyle::WeekTow => {
                const WEEK_NS: u64 = 604_800_000_000_000;
                let week = self.nanos / WEEK_NS;
                let tow_ns = self.nanos % WEEK_NS;
                let tow_s = tow_ns / 1_000_000_000;
                let tow_ms = (tow_ns % 1_000_000_000) / 1_000_000;

                write!(f, "{} {}:{:06}.{:03}", S::NAME, week, tow_s, tow_ms)
            }
            DisplayStyle::DayTod => {
                const DAY_NS: u64 = 86_400_000_000_000;
                let day = self.nanos / DAY_NS;
                let tod_ns = self.nanos % DAY_NS;
                let tod_s = tod_ns / 1_000_000_000;
                let tod_ms = (tod_ns % 1_000_000_000) / 1_000_000;

                write!(f, "{} {}:{:05}.{:03}", S::NAME, day, tod_s, tod_ms)
            }
            DisplayStyle::Simple => {
                let secs = self.nanos / 1_000_000_000;
                let ns_rem = self.nanos % 1_000_000_000;

                write!(f, "{} +{}s {}ns", S::NAME, secs, ns_rem)
            }
        }
    }
}

// defmt support

#[cfg(feature = "defmt")]
impl<S: TimeScale> defmt::Format for Time<S> {
    fn format(
        &self,
        f: defmt::Formatter,
    ) {
        match S::DISPLAY_STYLE {
            DisplayStyle::WeekTow => {
                const WEEK_NS: u64 = 604_800_000_000_000;
                let week = self.nanos / WEEK_NS;
                let tow_ns = self.nanos % WEEK_NS;
                let tow_s = tow_ns / 1_000_000_000;
                let tow_ms = (tow_ns % 1_000_000_000) / 1_000_000;

                defmt::write!(f, "{} {}:{:06}.{:03}", S::NAME, week, tow_s, tow_ms)
            }
            DisplayStyle::DayTod => {
                const DAY_NS: u64 = 86_400_000_000_000;
                let day = self.nanos / DAY_NS;
                let tod_ns = self.nanos % DAY_NS;
                let tod_s = tod_ns / 1_000_000_000;
                let tod_ms = (tod_ns % 1_000_000_000) / 1_000_000;

                defmt::write!(f, "{} {}:{:05}.{:03}", S::NAME, day, tod_s, tod_ms)
            }
            DisplayStyle::Simple => {
                let secs = self.nanos / 1_000_000_000;
                let ns_rem = self.nanos % 1_000_000_000;

                defmt::write!(f, "{} +{}s {}ns", S::NAME, secs, ns_rem)
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use std::format;
    #[allow(unused_imports)]
    use std::string::ToString;
    #[allow(unused_imports)]
    use std::vec;

    use super::*;
    use crate::scale::{Beidou, Galileo, Glonass, Gps, Tai, Utc};

    #[test]
    fn test_size_equals_u64() {
        assert_eq!(core::mem::size_of::<Time<Gps>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Glonass>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Galileo>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Beidou>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Utc>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Tai>>(), 8);
    }

    #[test]
    fn test_epoch_is_zero() {
        assert_eq!(Time::<Gps>::EPOCH.as_nanos(), 0);
        assert_eq!(Time::<Utc>::EPOCH.as_nanos(), 0);
    }

    #[test]
    fn test_from_week_tow_zero() {
        let t = Time::<Gps>::from_week_tow(
            0,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t, Time::<Gps>::EPOCH);
    }

    #[test]
    fn test_from_week_tow_roundtrip() {
        let t = Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t.week(), 2345);
        assert_eq!(t.tow_seconds(), 432_000);
        assert_eq!(t.sub_second_nanos(), 0);
    }

    #[test]
    fn test_from_week_tow_with_fractional() {
        let t = Time::<Gps>::from_week_tow(
            2300,
            DurationParts {
                seconds: 3661,
                nanos: 500_000_000,
            },
        )
        .unwrap();

        assert_eq!(t.week(), 2300);
        assert_eq!(t.tow_seconds(), 3661);
        assert_eq!(t.sub_second_nanos(), 500_000_000);
    }

    #[test]
    fn test_from_week_tow_invalid() {
        assert!(matches!(
            Time::<Gps>::from_week_tow(
                0,
                DurationParts {
                    seconds: 604_800,
                    nanos: 0
                }
            ),
            Err(GnssTimeError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_from_day_tod_zero() {
        let t = Time::<Glonass>::from_day_tod(
            0,
            DurationParts {
                seconds: 0,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t, Time::<Glonass>::EPOCH);
    }

    #[test]
    fn test_from_day_tod_roundtrip() {
        let t = Time::<Glonass>::from_day_tod(
            10_512,
            DurationParts {
                seconds: 43_200,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t.day(), 10_512);
        assert_eq!(t.tod_seconds(), 43_200);
    }

    #[test]
    fn test_from_day_tod_invalid() {
        assert!(matches!(
            Time::<Glonass>::from_day_tod(
                0,
                DurationParts {
                    seconds: 86_400,
                    nanos: 0
                }
            ),
            Err(GnssTimeError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_add_positive_duration() {
        let t = Time::<Gps>::from_seconds(100);

        assert_eq!((t + Duration::from_seconds(50)).as_seconds(), 150);
    }

    #[test]
    fn test_add_negative_duration_moves_back() {
        let t = Time::<Gps>::from_seconds(100);

        assert_eq!((t + Duration::from_nanos(-50_000_000_000)).as_seconds(), 50);
    }

    #[test]
    fn test_roundtrip_add_sub() {
        let t = Time::<Galileo>::from_seconds(1_000_000);
        let d = Duration::from_seconds(12_345);

        assert_eq!(t + d - d, t);
    }

    #[test]
    fn test_sub_times_positive() {
        let a = Time::<Gps>::from_seconds(200);
        let b = Time::<Gps>::from_seconds(100);

        assert_eq!((a - b).as_seconds(), 100);
    }

    #[test]
    fn test_sub_times_negative() {
        let a = Time::<Gps>::from_seconds(100);
        let b = Time::<Gps>::from_seconds(200);

        assert_eq!((a - b).as_seconds(), -100);
    }

    #[test]
    fn test_sub_same_is_zero() {
        let t = Time::<Gps>::from_seconds(42);

        assert!((t - t).is_zero());
    }

    #[test]
    #[should_panic]
    fn test_add_overflow_panics() {
        let _ = Time::<Gps>::MAX + Duration::ONE_NANOSECOND;
    }

    #[test]
    fn test_checked_add_overflow() {
        assert!(Time::<Gps>::MAX
            .checked_add(Duration::ONE_NANOSECOND)
            .is_none());
    }

    #[test]
    fn test_checked_sub_underflow() {
        assert!(Time::<Gps>::EPOCH
            .checked_sub_duration(Duration::ONE_NANOSECOND)
            .is_none());
    }

    #[test]
    fn test_saturating_add_clamps() {
        assert_eq!(
            Time::<Gps>::MAX.saturating_add(Duration::from_seconds(1)),
            Time::<Gps>::MAX
        );
    }

    #[test]
    fn test_gps_to_tai_adds_19s() {
        let gps = Time::<Gps>::from_seconds(100);
        let tai = gps.to_tai().unwrap();

        assert_eq!(tai.as_seconds(), 119);
    }

    #[test]
    fn test_tai_to_gps_subtracts_19s() {
        let tai = Time::<Tai>::from_seconds(119);
        let gps = Time::<Gps>::from_tai(tai).unwrap();

        assert_eq!(gps.as_seconds(), 100);
    }

    #[test]
    fn test_roundtrip_via_tai() {
        let original = Time::<Gps>::from_seconds(5_000_000);
        let back = Time::<Gps>::from_tai(original.to_tai().unwrap()).unwrap();

        assert_eq!(original, back);
    }

    #[test]
    fn test_gps_galileo_identity_via_tai() {
        // Same TAI offset → identical TAI instant → GPS→Galileo preserves nanoseconds
        let gps = Time::<Gps>::from_seconds(12_345);
        let gal = gps.try_convert::<Galileo>().unwrap();

        assert_eq!(gps.as_nanos(), gal.as_nanos());
    }

    #[test]
    fn test_gps_to_beidou_via_tai() {
        // GPS(100s) → TAI(119s) → BDT: 119 - 33 = 86s
        let gps = Time::<Gps>::from_seconds(100);
        let bdt = gps.try_convert::<Beidou>().unwrap();

        assert_eq!(bdt.as_seconds(), 86);
    }

    #[test]
    fn test_contextual_scale_to_tai_fails() {
        let glo = Time::<Glonass>::from_seconds(100);

        assert!(matches!(
            glo.to_tai(),
            Err(GnssTimeError::LeapSecondsRequired)
        ));
    }

    #[test]
    fn test_tai_to_contextual_fails() {
        let tai = Time::<Tai>::from_seconds(100);

        assert!(matches!(
            Time::<Utc>::from_tai(tai),
            Err(GnssTimeError::LeapSecondsRequired)
        ));
    }

    #[test]
    fn test_to_tai_overflow() {
        let t = Time::<Gps>::from_nanos(u64::MAX);

        assert!(matches!(t.to_tai(), Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_from_tai_underflow() {
        // TAI(0) - 19s offset → negative GPS time → overflow
        let tai = Time::<Tai>::from_nanos(0);

        assert!(matches!(
            Time::<Gps>::from_tai(tai),
            Err(GnssTimeError::Overflow)
        ));
    }

    #[test]
    fn test_gps_display_week_tow_format() {
        let t = Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t.to_string(), "GPS 2345:432000.000");
    }

    #[test]
    fn test_gps_display_epoch_is_week_0() {
        let s = Time::<Gps>::EPOCH.to_string();

        assert_eq!(s, "GPS 0:000000.000");
    }

    #[test]
    fn test_gps_display_tow_zero_padded() {
        // TOW = 1 second → should be displayed as 000001
        let t = Time::<Gps>::from_week_tow(
            1,
            DurationParts {
                seconds: 1,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t.to_string(), "GPS 1:000001.000");
    }

    #[test]
    fn test_gps_display_with_millis() {
        let t = Time::<Gps>::from_week_tow(
            100,
            DurationParts {
                seconds: 0,
                nanos: 500_000_000,
            },
        )
        .unwrap();

        assert_eq!(t.to_string(), "GPS 100:000000.500");
    }

    #[test]
    fn test_glonass_display_day_tod_format() {
        let t = Time::<Glonass>::from_day_tod(
            10_512,
            DurationParts {
                seconds: 43_200,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t.to_string(), "GLO 10512:43200.000");
    }

    #[test]
    fn test_glonass_display_epoch() {
        let s = Time::<Glonass>::EPOCH.to_string();

        assert_eq!(s, "GLO 0:00000.000");
    }

    #[test]
    fn test_galileo_display_week_format() {
        let s = Time::<Galileo>::EPOCH.to_string();

        assert!(s.starts_with("GAL "));
        assert!(s.contains(':'));
    }

    #[test]
    fn test_tai_display_simple_format() {
        let t = Time::<Tai>::from_seconds(1_000_000_000);
        let s = t.to_string();

        assert!(s.starts_with("TAI +"));
        assert!(s.contains("1000000000s"));
    }

    #[test]
    fn test_utc_display_simple_format() {
        let s = Time::<Utc>::EPOCH.to_string();

        assert!(s.starts_with("UTC +"));
    }

    #[test]
    fn test_debug_shows_scale_and_nanos() {
        let t = Time::<Glonass>::from_nanos(777);
        let s = format!("{t:?}");

        assert!(s.contains("GLO") && s.contains("777"));
    }

    #[test]
    fn test_ordering() {
        let t0 = Time::<Gps>::from_seconds(0);
        let t1 = Time::<Gps>::from_seconds(1);
        let t2 = Time::<Gps>::from_seconds(2);
        let mut v = vec![t2, t0, t1];

        v.sort();

        assert_eq!(v, vec![t0, t1, t2]);
    }

    #[test]
    fn test_glonass_day_accessor() {
        let t = Time::<Glonass>::from_day_tod(
            42,
            DurationParts {
                seconds: 3600,
                nanos: 0,
            },
        )
        .unwrap();

        assert_eq!(t.day(), 42);
        assert_eq!(t.tod_seconds(), 3600);
    }

    #[test]
    fn test_time_max_behavior() {
        let max = Time::<Gps>::MAX;
        let one_ns = Duration::ONE_NANOSECOND;

        // checked_add returns None on overflow
        assert!(max.checked_add(one_ns).is_none());

        // saturating_add clamps at MAX
        assert_eq!(max.saturating_add(one_ns), max);

        // try_add returns error on overflow
        assert!(max.try_add(one_ns).is_err());
    }

    #[test]
    fn test_max_is_u64_max() {
        assert_eq!(Time::<Gps>::MAX.as_nanos(), u64::MAX);
        assert_eq!(Time::<Glonass>::MAX.as_nanos(), u64::MAX);
        assert_eq!(Time::<Galileo>::MAX.as_nanos(), u64::MAX);
        assert_eq!(Time::<Beidou>::MAX.as_nanos(), u64::MAX);
        assert_eq!(Time::<Tai>::MAX.as_nanos(), u64::MAX);
        assert_eq!(Time::<Utc>::MAX.as_nanos(), u64::MAX);
    }

    #[test]
    fn test_nanos_per_year_is_correct() {
        let expected: u64 = 365 * 24 * 3_600 * 1_000_000_000;

        assert_eq!(Time::<Gps>::NANOS_PER_YEAR, expected);
    }

    #[test]
    fn test_max_covers_at_least_500_years() {
        let years = Time::<Gps>::MAX.as_nanos() / Time::<Gps>::NANOS_PER_YEAR;

        assert!(
            years >= 500,
            "MAX should cover at least 500 years, got {years}"
        );
    }

    #[test]
    fn test_checked_add_one_ns_before_max_succeeds() {
        let t = Time::<Gps>::from_nanos(u64::MAX - 1);
        let result = t.checked_add(Duration::from_nanos(1));

        assert_eq!(result, Some(Time::<Gps>::MAX));
    }

    #[test]
    fn test_checked_add_at_max_overflows() {
        assert!(Time::<Gps>::MAX
            .checked_add(Duration::from_nanos(1))
            .is_none());
    }

    #[test]
    fn test_checked_add_large_positive_overflows() {
        let t = Time::<Gps>::from_nanos(u64::MAX - 100);

        assert!(t.checked_add(Duration::from_seconds(1)).is_none());
    }

    #[test]
    fn test_checked_sub_one_ns_after_epoch_succeeds() {
        let t = Time::<Gps>::from_nanos(1);
        let result = t.checked_sub_duration(Duration::from_nanos(1));

        assert_eq!(result, Some(Time::<Gps>::EPOCH));
    }

    #[test]
    fn test_checked_sub_at_epoch_underflows() {
        assert!(Time::<Gps>::EPOCH
            .checked_sub_duration(Duration::from_nanos(1))
            .is_none());
    }

    #[test]
    fn test_checked_sub_large_amount_underflows() {
        let t = Time::<Gps>::from_nanos(50);

        assert!(t.checked_sub_duration(Duration::from_seconds(1)).is_none());
    }

    #[test]
    fn test_saturating_add_negative_clamps_at_epoch() {
        assert_eq!(
            Time::<Gps>::EPOCH.saturating_add(Duration::from_nanos(-1)),
            Time::<Gps>::EPOCH
        );
    }

    #[test]
    fn test_saturating_add_normal_value_works() {
        let t = Time::<Gps>::from_seconds(100);

        assert_eq!(
            t.saturating_add(Duration::from_seconds(50)),
            Time::<Gps>::from_seconds(150)
        );
    }

    #[test]
    fn test_saturating_sub_clamps_at_epoch() {
        assert_eq!(
            Time::<Gps>::EPOCH.saturating_sub_duration(Duration::from_nanos(1)),
            Time::<Gps>::EPOCH
        );
    }

    #[test]
    fn test_saturating_sub_normal_value_works() {
        let t = Time::<Gps>::from_seconds(100);

        assert_eq!(
            t.saturating_sub_duration(Duration::from_seconds(30)),
            Time::<Gps>::from_seconds(70)
        );
    }

    #[test]
    fn test_try_add_overflow_returns_err() {
        let result = Time::<Gps>::MAX.try_add(Duration::from_nanos(1));

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_try_add_valid_value_works() {
        let t = Time::<Gps>::from_seconds(1_000);
        let result = t.try_add(Duration::from_seconds(500)).unwrap();

        assert_eq!(result.as_seconds(), 1_500);
    }

    #[test]
    #[should_panic]
    fn test_add_operator_panics_at_max() {
        let _ = Time::<Gps>::MAX + Duration::from_nanos(1);
    }

    #[test]
    #[should_panic]
    fn test_sub_operator_panics_at_epoch() {
        let _ = Time::<Gps>::EPOCH - Duration::from_nanos(1);
    }

    #[test]
    fn test_checked_elapsed_zero_gives_zero_duration() {
        let t = Time::<Gps>::from_seconds(1_000);
        assert_eq!(t.checked_elapsed(t), Some(Duration::ZERO));
    }

    #[test]
    fn test_checked_elapsed_overflows_when_gap_exceeds_i64() {
        // MAX - EPOCH = u64::MAX nanoseconds; i64 can hold roughly half of this range
        // The difference u64::MAX fits into i128, but not into i64 → None
        let result = Time::<Gps>::MAX.checked_elapsed(Time::<Gps>::EPOCH);

        assert!(result.is_none(), "gap exceeds i64::MAX so must return None");
    }

    #[test]
    fn test_checked_elapsed_within_i64_range_works() {
        let a = Time::<Gps>::from_seconds(1_000_000);
        let b = Time::<Gps>::from_seconds(500_000);
        let elapsed = a.checked_elapsed(b).unwrap();

        assert_eq!(elapsed.as_seconds(), 500_000);
    }

    #[test]
    fn test_unix_seconds_roundtrip() {
        let unix = 1_600_000_000; // 2020-09-13
        let utc = Time::<Utc>::from_unix_seconds(unix).unwrap();

        assert_eq!(utc.as_unix_seconds(), unix);
    }

    #[test]
    fn test_unix_nanos_roundtrip() {
        let unix_ns = 1_600_000_000_123_456_789;
        let utc = Time::<Utc>::from_unix_nanos(unix_ns).unwrap();

        assert_eq!(utc.as_unix_nanos(), unix_ns);
    }

    #[test]
    fn test_gps_display_format() {
        let t = Time::<Gps>::from_week_tow(
            2345,
            DurationParts {
                seconds: 432_000,
                nanos: 0,
            },
        )
        .unwrap();
        assert_eq!(t.to_string(), "GPS 2345:432000.000");
    }

    #[test]
    fn test_saturating_add_clamps_at_max() {
        assert_eq!(
            Time::<Gps>::MAX.saturating_add(Duration::from_nanos(1)),
            Time::<Gps>::MAX
        );
    }

    #[test]
    fn test_utc_from_unix_seconds_zero_fails() {
        // Unix epoch (1970-01-01) is before UTC epoch (1972-01-01)
        assert!(matches!(
            Time::<Utc>::from_unix_seconds(0),
            Err(GnssTimeError::Overflow)
        ));
    }

    #[test]
    fn test_utc_from_unix_seconds_negative_fails() {
        assert!(matches!(
            Time::<Utc>::from_unix_seconds(-1),
            Err(GnssTimeError::Overflow)
        ));
    }

    #[test]
    fn test_utc_from_unix_seconds_just_before_utc_epoch_fails() {
        // One second before 1972-01-01
        assert!(matches!(
            Time::<Utc>::from_unix_seconds(63_071_999),
            Err(GnssTimeError::Overflow)
        ));
    }

    #[test]
    fn test_utc_from_unix_seconds_at_utc_epoch_gives_epoch() {
        // 1972-01-01 00:00:00 UTC = unix 63_072_000
        let utc = Time::<Utc>::from_unix_seconds(63_072_000).unwrap();
        assert_eq!(utc, Time::<Utc>::EPOCH);
    }

    #[test]
    fn test_utc_from_unix_seconds_roundtrip() {
        let unix_s: i64 = 1_700_000_000; // 2023-11-14
        let utc = Time::<Utc>::from_unix_seconds(unix_s).unwrap();
        assert_eq!(utc.as_unix_seconds(), unix_s);
    }

    #[test]
    fn test_utc_from_unix_seconds_known_date() {
        // 2024-01-01 00:00:00 UTC = Unix 1_704_067_200
        let unix_s: i64 = 1_704_067_200;
        let utc = Time::<Utc>::from_unix_seconds(unix_s).unwrap();
        assert_eq!(utc.as_unix_seconds(), unix_s);
    }

    #[test]
    fn test_utc_as_unix_seconds_at_epoch_equals_offset() {
        use crate::UTC_EPOCH_UNIX_OFFSET_S;
        assert_eq!(
            Time::<Utc>::EPOCH.as_unix_seconds(),
            UTC_EPOCH_UNIX_OFFSET_S
        );
        assert_eq!(Time::<Utc>::EPOCH.as_unix_seconds(), 63_072_000);
    }

    #[test]
    fn test_utc_as_unix_seconds_one_second_after_epoch() {
        let utc = Time::<Utc>::from_nanos(1_000_000_000); // 1 s after UTC epoch
        assert_eq!(utc.as_unix_seconds(), 63_072_001);
    }

    #[test]
    fn test_utc_from_unix_nanos_at_utc_epoch() {
        use crate::UTC_EPOCH_UNIX_OFFSET_NS;
        let utc = Time::<Utc>::from_unix_nanos(UTC_EPOCH_UNIX_OFFSET_NS).unwrap();
        assert_eq!(utc, Time::<Utc>::EPOCH);
    }

    #[test]
    fn test_utc_from_unix_nanos_zero_fails() {
        assert!(matches!(
            Time::<Utc>::from_unix_nanos(0),
            Err(GnssTimeError::Overflow)
        ));
    }

    #[test]
    fn test_utc_from_unix_nanos_one_ns_before_utc_epoch_fails() {
        assert!(matches!(
            Time::<Utc>::from_unix_nanos(63_072_000_000_000_000 - 1),
            Err(GnssTimeError::Overflow)
        ));
    }

    #[test]
    fn test_utc_from_unix_nanos_roundtrip() {
        let unix_ns: i64 = 1_700_000_000_123_456_789;
        let utc = Time::<Utc>::from_unix_nanos(unix_ns).unwrap();
        assert_eq!(utc.as_unix_nanos(), unix_ns);
    }

    #[test]
    fn test_utc_as_unix_nanos_at_epoch() {
        use crate::UTC_EPOCH_UNIX_OFFSET_NS;
        assert_eq!(Time::<Utc>::EPOCH.as_unix_nanos(), UTC_EPOCH_UNIX_OFFSET_NS);
        assert_eq!(Time::<Utc>::EPOCH.as_unix_nanos(), 63_072_000_000_000_000);
    }

    #[test]
    fn test_utc_as_unix_nanos_one_ns_after_epoch() {
        let utc = Time::<Utc>::from_nanos(1);
        assert_eq!(utc.as_unix_nanos(), 63_072_000_000_000_001);
    }

    #[test]
    fn test_utc_unix_seconds_and_nanos_consistent() {
        let unix_s: i64 = 1_600_000_000;
        let unix_ns: i64 = unix_s * 1_000_000_000;
        let from_s = Time::<Utc>::from_unix_seconds(unix_s).unwrap();
        let from_ns = Time::<Utc>::from_unix_nanos(unix_ns).unwrap();
        assert_eq!(from_s, from_ns);
    }

    #[test]
    fn test_utc_unix_nanos_sub_second_preserved() {
        let unix_ns: i64 = 1_700_000_000_500_000_000; // .5 s
        let utc = Time::<Utc>::from_unix_nanos(unix_ns).unwrap();
        // seconds part
        assert_eq!(utc.as_unix_seconds(), 1_700_000_000);
        // nanoseconds round-trip
        assert_eq!(utc.as_unix_nanos(), unix_ns);
    }

    #[test]
    fn test_gps_from_unix_seconds_at_gps_epoch() {
        let ls = LeapSeconds::builtin();
        // GPS epoch (1980-01-06) in Unix time = 315_964_800
        // At that moment GPS − UTC = 0
        let gps = Time::<Gps>::from_unix_seconds(315_964_800, ls).unwrap();
        assert_eq!(gps, Time::<Gps>::EPOCH);
    }

    #[test]
    fn test_gps_from_unix_seconds_before_utc_epoch_fails() {
        let ls = LeapSeconds::builtin();
        // Before 1972-01-01 (UTC epoch) → error in utc step
        assert!(Time::<Gps>::from_unix_seconds(0, ls).is_err());
    }

    #[test]
    fn test_gps_as_unix_seconds_at_gps_epoch() {
        let ls = LeapSeconds::builtin();
        assert_eq!(Time::<Gps>::EPOCH.as_unix_seconds(ls).unwrap(), 315_964_800);
    }

    #[test]
    fn test_gps_unix_seconds_roundtrip() {
        let ls = LeapSeconds::builtin();
        // 2020-01-01 00:00:00 UTC = Unix 1_577_836_800
        let unix_s: i64 = 1_577_836_800;
        let gps = Time::<Gps>::from_unix_seconds(unix_s, ls).unwrap();
        assert_eq!(gps.as_unix_seconds(ls).unwrap(), unix_s);
    }

    #[test]
    fn test_gps_unix_seconds_post_2017() {
        let ls = LeapSeconds::builtin();
        // 2023-01-01 00:00:00 UTC = Unix 1_672_531_200
        let unix_s: i64 = 1_672_531_200;
        let gps = Time::<Gps>::from_unix_seconds(unix_s, ls).unwrap();
        assert_eq!(gps.as_unix_seconds(ls).unwrap(), unix_s);
    }

    #[test]
    fn test_gps_unix_offset_is_18s_post_2017() {
        let ls = LeapSeconds::builtin();
        // In 2023, GPS − UTC = 18 s, so GPS seconds = unix − 315_964_800 + 18
        let unix_s: i64 = 1_672_531_200; // 2023-01-01 UTC
        let gps = Time::<Gps>::from_unix_seconds(unix_s, ls).unwrap();
        let expected_gps_s = (unix_s - 315_964_800 + 18) as u64;
        assert_eq!(gps.as_seconds(), expected_gps_s);
    }
}
