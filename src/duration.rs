//! # Duration
//!
//! A signed, nanosecond-resolution time interval with no attachment to any
//! particular [`TimeScale`](crate::scale::TimeScale).
//!
//! ## Design decisions
//!
//! - **`i64` nanoseconds** — covers ±292 years, more than enough for any
//!   realistic GNSS differential measurement.
//! - **No scale phantom** — a `Duration` is just an interval; it means the same
//!   thing regardless of which constellation produced it.
//! - **No allocations** — `Copy` type, works in `#[no_std]` environments.
//! - **Checked and saturating arithmetic** — you choose the overflow policy

use core::{
    fmt,
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

use crate::GnssTimeError;

const NANOS_PER_SECOND: i64 = 1_000_000_000;
const NANOS_PER_MILLI: i64 = 1_000_000;
const NANOS_PER_MICRO: i64 = 1_000;

/// A signed time interval measured in nanoseconds.
///
/// `Duration` is domain-agnostic: it represents the *difference* between two
/// instants and can be added to or subtracted from any [`crate::Time<S>`].
///
/// # Range
///
/// `i64` nanoseconds ==> approximately ±292 years.
///
/// # Examples
///
/// ```rust
/// use gnss_time::Duration;
///
/// let one_sec = Duration::from_seconds(1);
/// let half_sec = Duration::from_millis(500);
///
/// assert_eq!(one_sec - half_sec, half_sec);
///
/// let neg = -one_sec;
///
/// assert_eq!(neg.as_nanos(), -1_000_000_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[must_use = "Duration is a value type; ignoring it has no effect"]
#[repr(transparent)]
pub struct Duration(i64); // nanoseconds

impl Duration {
    /// Zero duration.
    pub const ZERO: Duration = Duration(0);

    /// Maximum representable duration (~292 years).
    pub const MAX: Duration = Duration(i64::MAX);

    /// Minimum representable duration (~ -292 years).
    pub const MIN: Duration = Duration(i64::MIN);

    /// One nanosecond.
    pub const ONE_NANOSECOND: Duration = Duration(1);

    /// One second expressed as a `Duration`.
    pub const ONE_SECOND: Duration = Duration(NANOS_PER_SECOND);

    /// Create from a raw nanosecond count. Prefer the types constructors below.
    #[inline(always)]
    pub const fn from_nanos(nanos: i64) -> Self {
        Duration(nanos)
    }

    /// Create from whole microseconds.
    #[inline]
    pub const fn from_micros(micros: i64) -> Self {
        Duration(micros * NANOS_PER_MICRO)
    }

    /// Create from whole milliseconds.
    #[inline]
    pub const fn from_millis(millis: i64) -> Self {
        Duration(millis * NANOS_PER_MILLI)
    }

    /// Create from whole seconds.
    #[inline]
    pub const fn from_seconds(secs: i64) -> Self {
        Duration(secs * NANOS_PER_SECOND)
    }

    /// Create from whole minutes.
    #[inline]
    pub const fn from_minutes(mins: i64) -> Self {
        Duration(mins * 60 * NANOS_PER_SECOND)
    }

    /// Create from whole hours.
    #[inline]
    pub const fn from_hours(hours: i64) -> Self {
        Duration(hours * 3_600 * NANOS_PER_SECOND)
    }

    /// Create from whole days.
    #[inline]
    pub const fn from_days(days: i64) -> Self {
        Duration(days * 86_400 * NANOS_PER_SECOND)
    }

    /// Create from microseconds, returning `None` on overflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_micros(micros: i64) -> Option<Self> {
        match micros.checked_mul(NANOS_PER_MICRO) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Create from milliseconds, returning `None` on overflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_millis(millis: i64) -> Option<Self> {
        match millis.checked_mul(NANOS_PER_MILLI) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }

    /// Create from whole seconds, returning `None` on overflow.
    #[inline]
    #[must_use = "returns None on overflow; check the result"]
    pub const fn checked_from_seconds(secs: i64) -> Option<Self> {
        match secs.checked_mul(NANOS_PER_SECOND) {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }
}

impl Duration {
    /// Raw nanosecond count (may be negative).
    #[inline(always)]
    #[must_use]
    pub const fn as_nanos(self) -> i64 {
        self.0
    }

    /// Whole microseconds (truncated toward zero).
    #[inline]
    #[must_use]
    pub const fn as_micros(self) -> i64 {
        self.0 / NANOS_PER_MICRO
    }

    /// Whole millisecond (truncated toward zero).
    #[inline]
    #[must_use]
    pub const fn as_millis(self) -> i64 {
        self.0 / NANOS_PER_MILLI
    }

    /// Whole seconds (truncated toward zero).
    #[inline]
    #[must_use]
    pub const fn as_seconds(self) -> i64 {
        self.0 / NANOS_PER_SECOND
    }

    /// Floating-point seconds. Precision is limited by `f64` mantissa (~15
    /// significant decimal digits), sufficient for sub-nanosecond accuracy up
    /// to ~10 000 seconds.
    #[inline]
    #[must_use]
    pub fn as_seconds_f64(self) -> f64 {
        self.0 as f64 / NANOS_PER_SECOND as f64
    }

    /// Returns `true` if the duration is positive (> 0).
    #[inline]
    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Returns `true` if the duration is negative (< 0).
    #[inline]
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Returns `true` if the duration is zero.
    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Absolute value. Returns `None` for `Duration::MIN` (no positive
    /// counterpart in `i64`).
    #[inline]
    #[must_use = "returns None for Duration::MIN; check the result"]
    pub const fn abs(self) -> Option<Self> {
        match self.0.checked_abs() {
            Some(n) => Some(Duration(n)),
            None => None,
        }
    }
}

impl Duration {
    /// Add, returning `None` on overflow.
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

    /// Subtract, returning `None` on overflow.
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

    /// Add, saturating at `i64::MAX` / `i64::MIN`.
    #[inline]
    #[must_use = "saturating_add returns a new Duration; the original is unchanged"]
    pub const fn saturating_add(
        self,
        rhs: Duration,
    ) -> Duration {
        Duration(self.0.saturating_add(rhs.0))
    }

    /// Subtract, saturating at `i64::MAX` / `i64::MIN`.
    #[inline]
    #[must_use = "saturating_sub returns a new Duration; the original is unchanged"]
    pub const fn saturating_sub(
        self,
        rhs: Duration,
    ) -> Duration {
        Duration(self.0.saturating_sub(rhs.0))
    }

    /// Fallible addition - returns [`GnssTimeError::Overflow`] on overflow.
    #[inline]
    pub fn try_add(
        self,
        rhs: Duration,
    ) -> Result<Duration, GnssTimeError> {
        self.checked_add(rhs).ok_or(GnssTimeError::Overflow)
    }

    /// Fallible subtraction — returns [`GnssTimeError::Overflow`] on overflow.
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

    /// # Panics / Overflow
    ///
    /// In debug builds, panics on overflow.
    /// In release builds, wraps around (same semantics as `i64`).
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
        self.0 += rhs.0
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
        self.0 -= rhs.0
    }
}

impl Neg for Duration {
    type Output = Duration;

    #[inline]
    fn neg(self) -> Self::Output {
        Duration(-self.0)
    }
}

impl fmt::Display for Duration {
    /// Formats as `[−]Xs Ynano_s` preserving full precision.
    ///
    /// Examples: `"1s 0ns"`, `"-3s 141592654ns"`, `"0s 500000000ns"`.
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let total = self.0;
        let sign = if total < 0 { "-" } else { "" };
        let abs = total.unsigned_abs(); // u64
        let secs = abs / 1_000_000_000u64;
        let nanos = abs % 1_000_000_000u64;

        write!(f, "{sign}{secs}s {nanos}ns")
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
        let d = Duration::from_nanos(123456789);

        assert_eq!(d.as_nanos(), 123456789);
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

        // важно: trunc toward zero
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
}
