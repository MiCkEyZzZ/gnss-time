//! # Duration
//!
//! A signed, fixed-precision time interval with nanosecond resolution.
//!
//! `Duration` represents the difference between two instants and is
//! independent of any time scale, epoch, or calendar system.
//!
//! This type is a thin wrapper around a signed 64-bit integer storing a
//! count of nanoseconds.
//!
//! ## Representation
//!
//! Internally represented as an `i64` number of nanoseconds.
//!
//! ## Range
//!
//! Approximately ±292 years (`i64::MIN..=i64::MAX` nanoseconds).
//!
//! ## Semantics
//!
//! - Linear, uniform time (no leap seconds, no calendar irregularities)
//! - Arithmetic is performed in integer nanoseconds
//! - Negative durations are fully supported
//!
//! ## Guarantees
//!
//! - `#[repr(transparent)]` over `i64`
//! - `Copy`, `Clone`, `Eq`, `Ord`, `Hash`
//! - `no_std` compatible
//! - No hidden allocations
//!
//! ## Arithmetic
//!
//! - Operator-based arithmetic (`Add`, `Sub`, etc.) does **not** check for
//!   overflow
//! - Checked variants (`checked_*`) return `None` on overflow
//! - Saturating variants clamp to [`Duration::MIN`] / [`Duration::MAX`]
//! - Fallible variants return [`GnssTimeError`]
//!
//! ## Notes
//!
//! This type intentionally does **not** model:
//!
//! - Calendar units (months, years)
//! - Non-uniform days (leap seconds, DST)
//! - Any time scale (TAI, UTC, GPS, etc.)
//!
//! For such concepts, use higher-level types.

use core::{
    fmt,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

use crate::GnssTimeError;

/// The number of nanoseconds in one second.
const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// The number of nanoseconds in one millisecond.
const NANOS_PER_MILLI: i64 = 1_000_000;

/// The number of nanoseconds in one microsecond.
const NANOS_PER_MICRO: i64 = 1_000;

/// A signed time interval with nanosecond precision.
///
/// `Duration` is a value type representing a span of time, stored as a
/// signed 64-bit count of nanoseconds.
///
/// ## Precision
///
/// 1 nanosecond.
///
/// ## Range
///
/// Approximately ±292 years.
///
/// ## Examples
///
/// ```rust
/// use gnss_time::Duration;
///
/// let a = Duration::from_seconds(1);
/// let b = Duration::from_millis(500);
///
/// assert_eq!(a - b, b);
///
/// let neg = -a;
/// assert_eq!(neg.as_nanos(), -1_000_000_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[must_use = "Duration is a value type; ignoring it has no effect"]
#[repr(transparent)]
pub struct Duration(i64); // nanoseconds

impl Duration {
    /// Zero duration.
    pub const ZERO: Duration = Duration(0);

    /// Maximum representable duration.
    pub const MAX: Duration = Duration(i64::MAX);

    /// Minimum representable duration.
    pub const MIN: Duration = Duration(i64::MIN);

    /// One nanosecond.
    pub const ONE_NANOSECOND: Duration = Duration(1);

    /// One second.
    pub const ONE_SECOND: Duration = Duration(NANOS_PER_SECOND);

    /// Creates a `Duration` from nanoseconds.
    #[inline]
    pub const fn from_nanos(nanos: i64) -> Self {
        Duration(nanos)
    }

    /// Creates a `Duration` from microseconds.
    ///
    /// # Panics
    ///
    /// Panics if the result overflows `i64` (`|micros| >
    /// 9_223_372_036_854_775`). Use [`Duration::checked_from_micros`] for
    /// fallible construction.
    #[inline]
    pub const fn from_micros(micros: i64) -> Self {
        match micros.checked_mul(NANOS_PER_MICRO) {
            Some(n) => Duration(n),
            None => panic!("Duration::from_micros: overflow"),
        }
    }

    /// Creates a `Duration` from milliseconds.
    ///
    /// # Panics
    ///
    /// Panics if the result overflows `i64` (`|millis| > 9_223_372_036_854`).
    /// Use [`Duration::checked_from_millis`] for fallible construction.
    #[inline]
    pub const fn from_millis(millis: i64) -> Self {
        match millis.checked_mul(NANOS_PER_MILLI) {
            Some(n) => Duration(n),
            None => panic!("Duration::from_millis: overflow"),
        }
    }

    /// Creates a `Duration` from seconds.
    ///
    /// # Panics
    ///
    /// Panics if the result overflows `i64` (`|secs| > 9_223_372_036`).
    /// Use [`Duration::checked_from_seconds`] for fallible construction.
    #[inline]
    pub const fn from_seconds(secs: i64) -> Self {
        match secs.checked_mul(NANOS_PER_SECOND) {
            Some(n) => Duration(n),
            None => panic!("Duration::from_seconds: overflow"),
        }
    }

    /// Creates a `Duration` from minutes.
    ///
    /// # Panics
    ///
    /// Panics if the result overflows `i64` (`|mins| > 153_722_867`).
    /// Use [`Duration::checked_from_minutes`] for fallible construction.
    #[inline]
    pub const fn from_minutes(mins: i64) -> Self {
        match mins.checked_mul(60 * NANOS_PER_SECOND) {
            Some(n) => Duration(n),
            None => panic!("Duration::from_minutes: overflow"),
        }
    }

    /// Creates a `Duration` from hours.
    ///
    /// # Panics
    ///
    /// Panics if the result overflows `i64` (`|hours| > 2_562_047`).
    /// Use [`Duration::checked_from_hours`] for fallible construction.
    #[inline]
    pub const fn from_hours(hours: i64) -> Self {
        match hours.checked_mul(3_600 * NANOS_PER_SECOND) {
            Some(n) => Duration(n),
            None => panic!("Duration::from_hours: overflow"),
        }
    }

    /// Creates a `Duration` from days.
    ///
    /// # Panics
    ///
    /// Panics if the result overflows `i64` (`|days| > 106_751`).
    /// Use [`Duration::checked_from_days`] for fallible construction.
    #[inline]
    pub const fn from_days(days: i64) -> Self {
        match days.checked_mul(86_400 * NANOS_PER_SECOND) {
            Some(n) => Duration(n),
            None => panic!("Duration::from_days: overflow"),
        }
    }

    /// Creates a `Duration` from microseconds, returning `None` on overflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_micros(micros: i64) -> Option<Self> {
        match micros.checked_mul(NANOS_PER_MICRO) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Creates a `Duration` from milliseconds, returning `None` on overflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_millis(millis: i64) -> Option<Self> {
        match millis.checked_mul(NANOS_PER_MILLI) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Creates a `Duration` from seconds, returning `None` on overflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_seconds(secs: i64) -> Option<Self> {
        match secs.checked_mul(NANOS_PER_SECOND) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Creates a `Duration` from minutes, returning `None` on overflow.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// assert_eq!(
    ///     Duration::checked_from_minutes(60),
    ///     Some(Duration::from_nanos(3_600_000_000_000))
    /// );
    /// assert!(Duration::checked_from_minutes(i64::MAX).is_none());
    /// ```
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_minutes(mins: i64) -> Option<Self> {
        match mins.checked_mul(60 * NANOS_PER_SECOND) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Creates a `Duration` from hours, returning `None` on overflow.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// assert_eq!(
    ///     Duration::checked_from_hours(1),
    ///     Some(Duration::from_nanos(3_600_000_000_000))
    /// );
    /// assert!(Duration::checked_from_hours(i64::MAX).is_none());
    /// ```
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_hours(hours: i64) -> Option<Self> {
        match hours.checked_mul(3_600 * NANOS_PER_SECOND) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Creates a `Duration` from days, returning `None` on overflow.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// assert_eq!(
    ///     Duration::checked_from_days(1),
    ///     Some(Duration::from_nanos(86_400_000_000_000))
    /// );
    /// assert!(Duration::checked_from_days(i64::MAX).is_none());
    /// ```
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_days(days: i64) -> Option<Self> {
        match days.checked_mul(86_400 * NANOS_PER_SECOND) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Returns the raw nanosecond value.
    #[inline]
    #[must_use]
    pub const fn as_nanos(self) -> i64 {
        self.0
    }

    /// Returns whole microseconds (truncated toward zero).
    #[inline]
    #[must_use]
    pub const fn as_micros(self) -> i64 {
        self.0 / NANOS_PER_MICRO
    }

    /// Returns whole milliseconds (truncated toward zero).
    #[inline]
    #[must_use]
    pub const fn as_millis(self) -> i64 {
        self.0 / NANOS_PER_MILLI
    }

    /// Returns whole seconds (truncated toward zero).
    #[inline]
    #[must_use]
    pub const fn as_seconds(self) -> i64 {
        self.0 / NANOS_PER_SECOND
    }

    /// Returns seconds as `f64`.
    ///
    /// # Precision
    ///
    /// May lose precision for large durations (> ~2^53 nanoseconds ≈ 104 days).
    #[inline]
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn as_seconds_f64(self) -> f64 {
        self.0 as f64 / NANOS_PER_SECOND as f64
    }

    /// Returns `true` if positive.
    #[inline]
    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Returns `true` if negative.
    #[inline]
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Returns `true` if zero.
    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Returns absolute value, or `None` on overflow.
    #[inline]
    #[must_use = "returns None for Duration::MIN; check the result"]
    pub const fn abs(self) -> Option<Self> {
        match self.0.checked_abs() {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Checked addition.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_add(
        self,
        rhs: Duration,
    ) -> Option<Duration> {
        match self.0.checked_add(rhs.0) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Checked subtraction.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_sub(
        self,
        rhs: Duration,
    ) -> Option<Duration> {
        match self.0.checked_sub(rhs.0) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Saturating addition.
    #[inline]
    #[must_use = "saturating_add returns a new Duration; the original is unchanged"]
    pub const fn saturating_add(
        self,
        rhs: Duration,
    ) -> Duration {
        Duration(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction.
    #[inline]
    #[must_use = "saturating_sub returns a new Duration; the original is unchanged"]
    pub const fn saturating_sub(
        self,
        rhs: Duration,
    ) -> Duration {
        Duration(self.0.saturating_sub(rhs.0))
    }

    /// Fallible addition.
    ///
    /// # Errors
    ///
    /// Returns [`GnssTimeError::Overflow`] if the result cannot be represented.
    #[inline]
    pub fn try_add(
        self,
        rhs: Duration,
    ) -> Result<Duration, GnssTimeError> {
        self.checked_add(rhs).ok_or(GnssTimeError::Overflow)
    }

    /// Fallible subtraction.
    ///
    /// # Errors
    ///
    /// Returns [`GnssTimeError::Overflow`] if the result cannot be represented.
    #[inline]
    pub fn try_sub(
        self,
        rhs: Duration,
    ) -> Result<Duration, GnssTimeError> {
        self.checked_sub(rhs).ok_or(GnssTimeError::Overflow)
    }
}

impl Add for Duration {
    type Output = Duration;

    #[inline]
    fn add(
        self,
        rhs: Self,
    ) -> Self::Output {
        Duration(self.0 + rhs.0)
    }
}

impl AddAssign for Duration {
    #[inline]
    fn add_assign(
        &mut self,
        rhs: Self,
    ) {
        self.0 += rhs.0;
    }
}

impl Sub for Duration {
    type Output = Duration;

    #[inline]
    fn sub(
        self,
        rhs: Self,
    ) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl SubAssign for Duration {
    #[inline]
    fn sub_assign(
        &mut self,
        rhs: Self,
    ) {
        self.0 -= rhs.0;
    }
}

impl Neg for Duration {
    type Output = Duration;

    #[inline]
    fn neg(self) -> Self::Output {
        Duration(-self.0)
    }
}

impl FromStr for Duration {
    type Err = GnssTimeError;

    /// Parses `"<seconds>s <nanos>ns"`, the exact inverse of `Display`.
    ///
    /// The optional `-` sign (on the seconds field) belongs to the whole
    /// value: both fields are non-negative magnitudes combined as
    /// `seconds * 1_000_000_000 + nanos`, then the sign is applied. Any other
    /// form (e.g. a signed nanos field) is rejected.
    ///
    /// # Errors
    ///
    /// - [`GnssTimeError::ParseError`] for any structural mismatch (missing
    ///   `'s'`/`'ns'` suffix, missing separating space, non-numeric field,
    ///   negative nanos field).
    ///
    /// - [`GnssTimeError::Overflow`] if the resulting value does not fit into
    ///   `i64`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// let d: Duration = "1s 500000000ns".parse().unwrap();
    ///
    /// assert_eq!(d.as_nanos(), 1_500_000_000);
    ///
    /// let d2: Duration = "-1s 500000000ns".parse().unwrap();
    ///
    /// assert_eq!(d2.as_nanos(), -1_500_000_000);
    ///
    /// // Round-trip:
    /// assert_eq!(d.to_string(), "1s 500000000ns");
    /// assert_eq!(d.to_string().parse::<Duration>().unwrap(), d);
    /// assert_eq!(d2.to_string().parse::<Duration>().unwrap(), d2);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (secs_field, nanos_field) = s
            .split_once(' ')
            .ok_or(GnssTimeError::ParseError("expected '<seconds>s <nanos>ns'"))?;

        let secs_str = secs_field
            .strip_suffix('s')
            .ok_or(GnssTimeError::ParseError(
                "expected 's' suffix on seconds field",
            ))?;
        let nanos_str = nanos_field
            .strip_suffix("ns")
            .ok_or(GnssTimeError::ParseError(
                "expected 'ns' suffix on nanos field",
            ))?;

        if nanos_str.starts_with('-') {
            return Err(GnssTimeError::ParseError(
                "nanos field must be non-negative; the sign belongs to the whole value",
            ));
        }

        let (negative, secs_str) = match secs_str.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, secs_str),
        };

        let secs_mag: u64 = secs_str
            .parse()
            .map_err(|_| GnssTimeError::ParseError("invalid seconds value"))?;
        let nanos_mag: u64 = nanos_str
            .parse()
            .map_err(|_| GnssTimeError::ParseError("invalid nanoseconds value"))?;

        let magnitude = secs_mag
            .checked_mul(1_000_000_000)
            .and_then(|s| s.checked_add(nanos_mag))
            .ok_or(GnssTimeError::Overflow)?;

        if negative {
            let total = i128::from(magnitude)
                .checked_neg()
                .and_then(|v| i64::try_from(v).ok())
                .ok_or(GnssTimeError::Overflow)?;

            Ok(Duration::from_nanos(total))
        } else {
            let total = i64::try_from(magnitude).map_err(|_| GnssTimeError::Overflow)?;

            Ok(Duration::from_nanos(total))
        }
    }
}

impl From<Duration> for i64 {
    /// Returns the raw nanosecond count, identical to [`Duration::as_nanos`].
    ///
    /// Infallible: `Duration` as `i64` nanoseconds at the representation level,
    /// so this is a direct, lossless read of that value,
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// let d = Duration::from_seconds(-7);
    /// let n: i64 = d.into();
    /// assert_eq!(n, -7_000_000_000);
    /// ```
    fn from(value: Duration) -> Self {
        value.as_nanos()
    }
}

impl From<i64> for Duration {
    /// Constructs a `Duration` from a raw nanosecond count, identical to
    /// [`Duration::from_nanos`].
    ///
    /// Infallible: every `i64` value is a valid nanosecond count for
    /// `Duration` — there is no narrower range to violate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// let d: Duration = (-7_000_000_000_i64).into();
    /// assert_eq!(d, Duration::from_seconds(-7));
    /// ```
    fn from(value: i64) -> Self {
        Duration::from_nanos(value)
    }
}

impl fmt::Display for Duration {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let abs = self.0.unsigned_abs();
        let sign = if self.0 < 0 { "-" } else { "" };
        let secs = abs / 1_000_000_000;
        let nanos = abs % 1_000_000_000;

        write!(f, "{sign}{secs}s {nanos}ns")
    }
}

impl TryFrom<core::time::Duration> for Duration {
    type Error = GnssTimeError;

    /// Converts a `core::time::Duration` into a `gnss_time::Duration`.
    ///
    /// `core::time::Duration` is always non-negative, so the result is always
    /// non-negative too - this direction only fails on magnitude, never on
    /// sign.
    ///
    /// # Errors
    ///
    /// - [`GnssTimeError::Overflow`] if the input exceeds `i64::MAX`
    ///   nanoseconds (≈ 292 years) — `core::time::Duration` can represent
    ///   values far beyond `gnss_time::Duration`'s range.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// let std_d = core::time::Duration::from_secs(1);
    /// let d: Duration = std_d.try_into().unwrap();
    /// assert_eq!(d, Duration::from_seconds(1));
    ///
    /// // Sub-second precision is preserved:
    /// let std_d = core::time::Duration::from_nanos(123_456_789);
    /// let d: Duration = std_d.try_into().unwrap();
    /// assert_eq!(d.as_nanos(), 123_456_789);
    ///
    /// // A core::time::Duration beyond i64::MAX nanoseconds does not fit:
    /// let huge = core::time::Duration::from_secs(u64::MAX);
    /// assert!(Duration::try_from(huge).is_err());
    /// ```
    fn try_from(value: core::time::Duration) -> Result<Self, Self::Error> {
        let nanos = i64::try_from(value.as_nanos()).map_err(|_| GnssTimeError::Overflow)?;

        Ok(Duration::from_nanos(nanos))
    }
}

impl TryFrom<Duration> for core::time::Duration {
    type Error = GnssTimeError;

    /// Converts a `gnss_time::Duration` into a `core::time::Duration`.
    ///
    /// # Errors
    ///
    /// - [`GnssTimeError::OutOfRange`] if `value` is negative -
    ///   `core::time::Duration` cannot represent a negative interval, and there
    ///   is no honest clamping or truncation to fall back to. Use
    ///   [`Duration::abs`] first if you specifically want the magnitude:
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// let negative = Duration::from_seconds(-5);
    /// let magnitude: core::time::Duration = negative.abs().unwrap().try_into().unwrap();
    /// assert_eq!(magnitude, core::time::Duration::from_secs(5));
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::Duration;
    ///
    /// let d = Duration::from_seconds(1);
    /// let std_d: core::time::Duration = d.try_into().unwrap();
    /// assert_eq!(std_d, core::time::Duration::from_secs(1));
    ///
    /// // Negative durations have no representation in core::time::Duration:
    /// let negative = Duration::from_seconds(-1);
    /// assert!(core::time::Duration::try_from(negative).is_err());
    /// ```
    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        let nanos = u64::try_from(value.as_nanos()).map_err(|_| GnssTimeError::OutOfRange)?;

        Ok(core::time::Duration::from_nanos(nanos))
    }
}

// defmt support: embedded logging via probe-rs / defmt-rtt.
#[cfg(feature = "defmt")]
impl defmt::Format for Duration {
    #[allow(clippy::if_same_then_else)]
    fn format(
        &self,
        f: defmt::Formatter,
    ) {
        let abs = self.0.unsigned_abs();
        let secs = abs / 1_000_000_000;
        let nanos = abs % 1_000_000_000;

        // The two branches emit different format tags, but after defmt::write!
        // macro expansion the bodies look identical to clippy.
        if self.0 < 0 {
            defmt::write!(f, "-{}s {}ns", secs, nanos);
        } else {
            defmt::write!(f, "{}s {}ns", secs, nanos);
        }
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

    #[test]
    fn test_from_seconds_roundtrip() {
        let d = Duration::from_seconds(42);

        assert_eq!(d.as_seconds(), 42);
        assert_eq!(d.as_nanos(), 42_000_000_000);
    }

    #[test]
    fn test_from_millis_roundtrip() {
        let d = Duration::from_millis(1500);

        assert_eq!(d.as_millis(), 1500);
        assert_eq!(d.as_seconds(), 1);
    }

    #[test]
    fn test_from_micros_roundtrip() {
        let d = Duration::from_micros(1_000_000);

        assert_eq!(d.as_micros(), 1_000_000);
        assert_eq!(d.as_millis(), 1_000);
    }

    #[test]
    fn test_zero_constants() {
        assert!(Duration::ZERO.is_zero());
        assert_eq!(Duration::ZERO.as_nanos(), 0);
    }

    #[test]
    fn test_sign_helpers() {
        assert!(Duration::from_seconds(1).is_positive());
        assert!(Duration::from_seconds(-1).is_negative());
        assert!(!Duration::ZERO.is_positive());
        assert!(!Duration::ZERO.is_negative());
    }

    #[test]
    fn test_add_sub_identify() {
        let a = Duration::from_seconds(10);
        let b = Duration::from_seconds(3);

        assert_eq!(a - b + b, a);
    }

    #[test]
    fn test_negative() {
        let d = Duration::from_seconds(5);

        assert_eq!((-d).as_nanos(), -5_000_000_000);
        assert_eq!(-(-d), d);
    }

    #[test]
    fn test_checked_add_overflow() {
        assert!(Duration::MAX
            .checked_add(Duration::ONE_NANOSECOND)
            .is_none());
    }

    #[test]
    fn test_checked_add_underflow() {
        assert!(Duration::MIN
            .checked_sub(Duration::ONE_NANOSECOND)
            .is_none());
    }

    #[test]
    fn test_saturating_add_clamps() {
        let result = Duration::MAX.saturating_add(Duration::ONE_NANOSECOND);

        assert_eq!(result, Duration::MAX);
    }

    #[test]
    fn test_saturating_sub_clamps() {
        let result = Duration::MIN.saturating_sub(Duration::ONE_NANOSECOND);

        assert_eq!(result, Duration::MIN);
    }

    #[test]
    fn test_abs_positive() {
        let d = Duration::from_seconds(-7);

        assert_eq!(d.abs().unwrap().as_seconds(), 7);
    }

    #[test]
    fn test_abs_min_is_none() {
        assert!(Duration::MIN.abs().is_none());
    }

    #[test]
    fn test_as_seconds_f64_precision() {
        let d = Duration::from_nanos(1_500_000_001); // 1.500000001 s
        let f = d.as_seconds_f64();

        // f64 has ~15 significant digits; 1.500000001 requires 10 → represented exactly
        assert!((f - 1.500_000_001_f64).abs() < 1e-9);
    }

    #[test]
    fn test_display_positive() {
        assert_eq!(Duration::from_seconds(1).to_string(), "1s 0ns");
    }

    #[test]
    fn test_display_negative() {
        let d = Duration::from_nanos(-3_141_592_654);
        assert_eq!(d.to_string(), "-3s 141592654ns");
    }

    #[test]
    fn test_display_zero() {
        assert_eq!(Duration::ZERO.to_string(), "0s 0ns");
    }

    #[test]
    fn test_size_of_duration_is_8_bytes() {
        assert_eq!(core::mem::size_of::<Duration>(), 8);
    }

    #[test]
    fn test_identity_zero_addition() {
        let d = Duration::from_seconds(123);

        assert_eq!(d + Duration::ZERO, d);
        assert_eq!(Duration::ZERO + d, d);
    }

    #[test]
    fn test_identity_zero_subtraction() {
        let d = Duration::from_seconds(123);

        assert_eq!(d - Duration::ZERO, d);
    }

    #[test]
    fn test_double_negation() {
        let d = Duration::from_seconds(999);

        assert_eq!(-(-d), d);
    }

    #[test]
    fn test_add_sub_inverse() {
        let a = Duration::from_seconds(1000);
        let b = Duration::from_seconds(250);

        assert_eq!((a + b) - b, a);
    }

    #[test]
    fn test_sub_add_inverse() {
        let a = Duration::from_seconds(1000);
        let b = Duration::from_seconds(250);

        assert_eq!((a - b) + b, a);
    }

    #[test]
    fn test_add_commutativity() {
        let a = Duration::from_seconds(10);
        let b = Duration::from_seconds(3);

        assert_eq!(a + b, b + a);
    }

    #[test]
    fn test_add_associativity() {
        let a = Duration::from_seconds(1);
        let b = Duration::from_seconds(2);
        let c = Duration::from_seconds(3);

        assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn test_checked_add_matches_operator_when_safe() {
        let a = Duration::from_seconds(10);
        let b = Duration::from_seconds(5);

        assert_eq!(a.checked_add(b), Some(a + b));
    }

    #[test]
    fn test_checked_sub_matches_operator_when_safe() {
        let a = Duration::from_seconds(10);
        let b = Duration::from_seconds(5);

        assert_eq!(a.checked_sub(b), Some(a - b));
    }

    #[test]
    fn test_sign_symmetry() {
        let d = Duration::from_seconds(42);

        assert_eq!(d.is_positive(), (-d).is_negative());
        assert_eq!(d.is_negative(), (-d).is_positive());
    }

    #[test]
    fn test_conversion_consistency() {
        let d = Duration::from_seconds(1);

        assert_eq!(Duration::from_millis(1000), d);
        assert_eq!(Duration::from_micros(1_000_000), d);
    }

    #[test]
    fn test_nanos_identity() {
        let d = Duration::from_nanos(123_456_789);

        assert_eq!(d.as_nanos(), 123_456_789);
    }

    #[test]
    fn test_checked_from_seconds_overflow() {
        assert!(Duration::checked_from_seconds(i64::MAX / NANOS_PER_SECOND + 1).is_none());
    }

    #[test]
    fn test_checked_from_millis_overflow() {
        assert!(Duration::checked_from_millis(i64::MAX / NANOS_PER_MILLI + 1).is_none());
    }

    #[test]
    fn test_checked_from_micros_overflow() {
        assert!(Duration::checked_from_micros(i64::MAX / NANOS_PER_MICRO + 1).is_none());
    }

    #[test]
    fn test_as_seconds_truncation_positive() {
        let d = Duration::from_nanos(1_500_000_000);

        assert_eq!(d.as_seconds(), 1);
    }

    #[test]
    fn test_as_seconds_truncation_negative() {
        let d = Duration::from_nanos(-1_500_000_000);

        // note: truncating toward zero
        assert_eq!(d.as_seconds(), -1);
    }

    #[test]
    fn test_as_millis_truncation_negative() {
        let d = Duration::from_nanos(-1_500_000);
        assert_eq!(d.as_millis(), -1);
    }

    #[test]
    fn test_add_assign() {
        let mut d = Duration::from_seconds(10);
        d += Duration::from_seconds(5);

        assert_eq!(d, Duration::from_seconds(15));
    }

    #[test]
    fn test_sub_assign() {
        let mut d = Duration::from_seconds(10);
        d -= Duration::from_seconds(5);

        assert_eq!(d, Duration::from_seconds(5));
    }

    #[test]
    fn test_add_assign_zero_identity() {
        let mut d = Duration::from_seconds(42);
        d += Duration::ZERO;

        assert_eq!(d, Duration::from_seconds(42));
    }

    #[test]
    fn test_sub_assign_zero_identity() {
        let mut d = Duration::from_seconds(42);
        d -= Duration::ZERO;

        assert_eq!(d, Duration::from_seconds(42));
    }

    #[test]
    fn test_min_plus_zero() {
        assert_eq!(Duration::MIN + Duration::ZERO, Duration::MIN);
    }

    #[test]
    fn test_max_plus_zero() {
        assert_eq!(Duration::MAX + Duration::ZERO, Duration::MAX);
    }

    #[test]
    fn test_min_minus_zero() {
        assert_eq!(Duration::MIN - Duration::ZERO, Duration::MIN);
    }

    #[test]
    fn test_max_minus_zero() {
        assert_eq!(Duration::MAX - Duration::ZERO, Duration::MAX);
    }

    #[test]
    fn test_abs_positive_identity() {
        let d = Duration::from_seconds(10);
        assert_eq!(d.abs().unwrap(), d);
    }

    #[test]
    fn test_abs_zero() {
        assert_eq!(Duration::ZERO.abs().unwrap(), Duration::ZERO);
    }

    #[test]
    fn test_seconds_millis_consistency() {
        assert_eq!(Duration::from_seconds(1), Duration::from_millis(1000));
    }

    #[test]
    fn test_seconds_micros_consistency() {
        assert_eq!(Duration::from_seconds(1), Duration::from_micros(1_000_000));
    }

    #[test]
    fn test_seconds_nanos_consistency() {
        assert_eq!(
            Duration::from_seconds(1),
            Duration::from_nanos(1_000_000_000)
        );
    }

    #[test]
    fn test_checked_add_matches_manual() {
        let a = Duration::from_seconds(123);
        let b = Duration::from_seconds(456);

        assert_eq!(a.checked_add(b), Some(Duration::from_seconds(579)));
    }

    #[test]
    fn test_checked_sub_matches_manual() {
        let a = Duration::from_seconds(500);
        let b = Duration::from_seconds(200);

        assert_eq!(a.checked_sub(b), Some(Duration::from_seconds(300)));
    }

    #[test]
    fn test_ordering_basic() {
        let a = Duration::from_seconds(1);
        let b = Duration::from_seconds(2);

        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn test_ordering_zero() {
        let a = Duration::ZERO;
        let b = Duration::from_seconds(1);

        assert!(a < b);
    }

    #[test]
    fn test_neg_zero() {
        assert_eq!(-Duration::ZERO, Duration::ZERO);
    }

    #[test]
    fn test_neg_sign_flip() {
        let d = Duration::from_seconds(100);

        assert_eq!(-d, Duration::from_seconds(-100));
    }

    #[test]
    fn test_checked_add_overflow_returns_none() {
        assert_eq!(Duration::MAX.checked_add(Duration::ONE_NANOSECOND), None);
    }

    #[test]
    fn test_checked_sub_underflow_returns_none() {
        assert_eq!(Duration::MIN.checked_sub(Duration::ONE_NANOSECOND), None);
    }

    #[test]
    fn test_try_add_overflow_returns_err() {
        assert_eq!(
            Duration::MAX.try_add(Duration::ONE_NANOSECOND),
            Err(GnssTimeError::Overflow)
        );
    }

    #[test]
    fn test_try_sub_underflow_returns_err() {
        assert_eq!(
            Duration::MIN.try_sub(Duration::ONE_NANOSECOND),
            Err(GnssTimeError::Overflow)
        );
    }

    #[test]
    fn test_from_str_basic() {
        let d: Duration = "1s 500000000ns".parse().unwrap();

        assert_eq!(d.as_nanos(), 1_500_000_000);
    }

    #[test]
    fn test_from_str_negative() {
        let d: Duration = "-1s 500000000ns".parse().unwrap();

        assert_eq!(d.as_nanos(), -1_500_000_000);
    }

    #[test]
    fn test_from_str_negative_sub_second() {
        let d: Duration = "-0s 1ns".parse().unwrap();

        assert_eq!(d.as_nanos(), -1);
    }

    #[test]
    fn test_from_str_signed_nanos_field_errors() {
        let result: Result<Duration, _> = "-1s -500000000ns".parse();

        assert!(matches!(result, Err(GnssTimeError::ParseError(_))));
    }

    #[test]
    fn test_from_str_zero() {
        let d: Duration = "0s 0ns".parse().unwrap();

        assert_eq!(d, Duration::ZERO);
    }

    #[test]
    fn test_from_str_missing_space_errors() {
        let result: Result<Duration, _> = "1s500000000ns".parse();

        assert!(matches!(result, Err(GnssTimeError::ParseError(_))));
    }

    #[test]
    fn test_from_str_missing_s_suffix_errors() {
        let result: Result<Duration, _> = "1 500000000ns".parse();

        assert!(matches!(result, Err(GnssTimeError::ParseError(_))));
    }

    #[test]
    fn test_from_str_missing_ns_suffix_errors() {
        let result: Result<Duration, _> = "1s 500000000".parse();

        assert!(matches!(result, Err(GnssTimeError::ParseError(_))));
    }

    #[test]
    fn test_from_str_non_numeric_errors() {
        let result: Result<Duration, _> = "abcs 500000000ns".parse();

        assert!(matches!(result, Err(GnssTimeError::ParseError(_))));
    }

    #[test]
    fn test_from_str_overflow_errors() {
        let result: Result<Duration, _> = "9223372037s 0ns".parse();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_display_fromstr_roundtrip_many_values() {
        let cases: &[i64] = &[
            0,
            1,
            -1,
            999_999_999,
            -999_999_999,
            1_000_000_000,
            -1_000_000_000,
            1_500_000_000,
            -1_500_000_000,
            i64::MAX,
            i64::MIN,
        ];
        for &n in cases {
            let d = Duration::from_nanos(n);
            let s = d.to_string();
            let parsed: Duration = s.parse().unwrap_or_else(|e| {
                panic!("failed to parse Display output {s:?} for nanos={n}: {e:?}")
            });

            assert_eq!(d, parsed, "round-trip failed for nanos={n}, display={s:?}");
        }
    }

    #[test]
    fn test_display_fromstr_roundtrip_max() {
        let d = Duration::MAX;
        let s = d.to_string();
        let parsed: Duration = s.parse().unwrap();

        assert_eq!(d, parsed);
    }

    #[test]
    fn test_display_fromstr_roundtrip_min() {
        let d = Duration::MIN;
        let s = d.to_string();
        let parsed: Duration = s.parse().unwrap();

        assert_eq!(d, parsed);
    }

    #[test]
    fn test_from_std_one_second() {
        let std_d = core::time::Duration::from_secs(1);
        let d: Duration = std_d.try_into().unwrap();

        assert_eq!(d, Duration::from_seconds(1));
    }

    #[test]
    fn test_from_std_zero() {
        let std_d = core::time::Duration::ZERO;
        let d: Duration = std_d.try_into().unwrap();

        assert_eq!(d, Duration::ZERO);
    }

    #[test]
    fn test_from_std_preserves_sub_second_nanos() {
        let std_d = core::time::Duration::from_nanos(123_456_789);
        let d: Duration = std_d.try_into().unwrap();

        assert_eq!(d.as_nanos(), 123_456_789);
    }

    #[test]
    fn test_from_std_preserves_mixed_secs_and_nanos() {
        let std_d = core::time::Duration::new(5, 500_000_000);
        let d: Duration = std_d.try_into().unwrap();

        assert_eq!(d.as_nanos(), 5_500_000_000);
    }

    #[test]
    fn test_from_std_max_i64_nanos_succeeds() {
        // i64::MAX nanoseconds is exactly representable
        let std_d = core::time::Duration::from_nanos(i64::MAX as u64);
        let d: Duration = std_d.try_into().unwrap();

        assert_eq!(d.as_nanos(), i64::MAX);
    }

    #[test]
    fn test_from_std_beyond_i64_max_overflows() {
        // i64::MAX + 1 nanoseconds does not fit
        let std_d = core::time::Duration::from_nanos(i64::MAX as u64 + 1);
        let result: Result<Duration, _> = std_d.try_into();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_from_std_u64_max_secs_overflows() {
        let std_d = core::time::Duration::from_secs(u64::MAX);
        let result: Result<Duration, _> = std_d.try_into();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_from_std_is_never_negative() {
        // core::time::Duration cannot be negative, so this direction never fails for
        // sign reason - only ever for magnitude (testes above).
        let std_d = core::time::Duration::from_nanos(1);
        let d: Duration = std_d.try_into().unwrap();

        assert!(!d.is_negative());
    }

    #[test]
    fn test_to_std_one_second() {
        let d = Duration::from_seconds(1);
        let std_d: core::time::Duration = d.try_into().unwrap();

        assert_eq!(std_d, core::time::Duration::from_secs(1));
    }

    #[test]
    fn test_to_std_zero() {
        let d = Duration::ZERO;
        let std_d: core::time::Duration = d.try_into().unwrap();

        assert_eq!(std_d, core::time::Duration::ZERO);
    }

    #[test]
    fn test_to_std_preserves_sub_second_nanos() {
        let d = Duration::from_nanos(123_456_789);
        let std_d: core::time::Duration = d.try_into().unwrap();

        assert_eq!(std_d.as_nanos(), 123_456_789);
    }

    #[test]
    fn test_to_std_negative_errors() {
        let d = Duration::from_seconds(-1);
        let result: Result<core::time::Duration, _> = d.try_into();

        assert!(matches!(result, Err(GnssTimeError::OutOfRange)));
    }

    #[test]
    fn test_to_std_negative_one_nanosecond_errors() {
        // Boundary: even the smallest possible negative value must fail,
        // not just "large" negative values.
        let d = Duration::from_nanos(-1);
        let result: Result<core::time::Duration, _> = d.try_into();

        assert!(matches!(result, Err(GnssTimeError::OutOfRange)));
    }

    #[test]
    fn test_to_std_max_succeeds() {
        let d = Duration::MAX;
        let std_d: core::time::Duration = d.try_into().unwrap();

        assert_eq!(std_d.as_nanos(), i64::MAX as u128);
    }

    #[test]
    fn test_to_std_min_errors() {
        // Duration::MIN is negative by construction (see I-5)
        let d = Duration::MIN;
        let result: Result<core::time::Duration, _> = d.try_into();

        assert!(matches!(result, Err(GnssTimeError::OutOfRange)));
    }

    #[test]
    fn test_to_std_negative_via_abs_succeeds() {
        let negative = Duration::from_seconds(-5);
        let magnitude: core::time::Duration = negative.abs().unwrap().try_into().unwrap();

        assert_eq!(magnitude, core::time::Duration::from_secs(5));
    }

    #[test]
    fn test_roundtrip_std_to_ours_to_std() {
        let original = core::time::Duration::new(12_345, 678_901_234);
        let ours: Duration = original.try_into().unwrap();
        let back: core::time::Duration = ours.try_into().unwrap();

        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_ours_to_std_to_ours_nonnegative() {
        let cases = [
            Duration::ZERO,
            Duration::from_seconds(1),
            Duration::from_nanos(1),
            Duration::MAX,
            Duration::from_nanos(999_999_999),
        ];

        for original in cases {
            let std_d: core::time::Duration = original.try_into().unwrap();
            let back: Duration = std_d.try_into().unwrap();

            assert_eq!(original, back, "round-trip failed for {original:?}");
        }
    }

    #[test]
    fn test_duration_to_i64() {
        let d = Duration::from_seconds(-7);
        let n: i64 = d.into();

        assert_eq!(n, -7_000_000_000);
    }

    #[test]
    fn test_from_i64_to_duration() {
        let d: Duration = (-7_000_000_000_i64).into();

        assert_eq!(d, Duration::from_seconds(-7));
    }

    #[test]
    fn test_i64_roundtrip() {
        let cases: &[i64] = &[0, 1, -1, i64::MAX, i64::MIN, 1_500_000_000, -1_500_000_000];

        for &n in cases {
            let d: Duration = n.into();
            let back: i64 = d.into();

            assert_eq!(n, back);
        }
    }

    #[test]
    fn test_i64_from_covers_full_range() {
        let min_d: Duration = i64::MIN.into();
        let max_d: Duration = i64::MAX.into();

        assert_eq!(min_d.as_nanos(), i64::MIN);
        assert_eq!(max_d.as_nanos(), i64::MAX);
    }
}
