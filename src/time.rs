use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::{Duration, Glonass, GnssTimeError, Gps, OffsetToTai, Tai, TimeScale};

/// A timestamp in time scale `S`, stored as nanoseconds since the scale's epoch.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,
}

impl<S: TimeScale> Time<S> {
    pub const EPOCH: Self = Time {
        nanos: 0,
        _scale: PhantomData,
    };

    pub const MAX: Self = Time {
        nanos: u64::MAX,
        _scale: PhantomData,
    };

    #[inline(always)]
    pub const fn from_nanos(nanos: u64) -> Self {
        Time {
            nanos,
            _scale: PhantomData,
        }
    }

    #[inline]
    pub const fn from_seconds(secs: u64) -> Self {
        Time::from_nanos(secs * 1_000_000_000)
    }

    #[inline]
    pub const fn checked_from_seconds(secs: u64) -> Option<Self> {
        match secs.checked_mul(1_000_000_000) {
            Some(n) => Some(Time::from_nanos(n)),
            None => None,
        }
    }
}

impl<S: TimeScale> Time<S> {
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

    pub fn try_convert<T: TimeScale>(self) -> Result<Time<T>, GnssTimeError> {
        let tai = self.to_tai()?;

        Time::<T>::from_tai(tai)
    }
}

impl<S: TimeScale> Time<S> {
    #[inline(always)]
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    #[inline]
    pub const fn as_seconds(self) -> u64 {
        self.nanos / 1_000_000_000
    }

    #[inline]
    pub const fn as_seconds_f64(self) -> f64 {
        self.nanos as f64 / 1_000_000_000.0
    }
}

impl<S: TimeScale> Time<S> {
    #[inline]
    pub fn checked_add(self, d: Duration) -> Option<Self> {
        let result = (self.nanos as i128) + (d.as_nanos() as i128);

        if result < 0 || result > u64::MAX as i128 {
            return None;
        };

        Some(Time::from_nanos(result as u64))
    }

    #[inline]
    pub fn checked_sub_duration(self, d: Duration) -> Option<Self> {
        let result = (self.nanos as i128) - (d.as_nanos() as i128);

        if result < 0 || result > u64::MAX as i128 {
            return None;
        }

        Some(Time::from_nanos(result as u64))
    }

    #[inline]
    pub fn saturating_add(self, d: Duration) -> Self {
        self.checked_add(d).unwrap_or(if d.is_negative() {
            Time::EPOCH
        } else {
            Time::MAX
        })
    }

    #[inline]
    pub fn saturating_sub_duration(self, d: Duration) -> Self {
        self.checked_sub_duration(d).unwrap_or(if d.is_negative() {
            Time::MAX
        } else {
            Time::EPOCH
        })
    }

    #[inline]
    pub fn try_add(self, d: Duration) -> Result<Self, GnssTimeError> {
        self.checked_add(d).ok_or(GnssTimeError::Overflow)
    }

    #[inline]
    pub fn try_sub_duration(self, d: Duration) -> Result<Self, GnssTimeError> {
        self.checked_sub_duration(d).ok_or(GnssTimeError::Overflow)
    }
}

impl<S: TimeScale> Time<S> {
    #[inline]
    pub const fn checked_elapsed(self, ealier: Time<S>) -> Option<Duration> {
        let diff = (self.nanos as i128) - (ealier.nanos as i128);

        if diff > i64::MAX as i128 || diff < i64::MIN as i128 {
            return None;
        }

        Some(Duration::from_nanos(diff as i64))
    }
}

impl<S: TimeScale> Add<Duration> for Time<S> {
    type Output = Time<S>;

    #[inline]
    fn add(self, rhs: Duration) -> Time<S> {
        self.checked_add(rhs)
            .expect("Time<S> + Duration overflowed")
    }
}

impl<S: TimeScale> AddAssign<Duration> for Time<S> {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs
    }
}

impl<S: TimeScale> Sub<Duration> for Time<S> {
    type Output = Time<S>;

    #[inline]
    fn sub(self, rhs: Duration) -> Self::Output {
        self.checked_sub_duration(rhs)
            .expect("Time<S> - Duration underflowed")
    }
}

impl<S: TimeScale> SubAssign<Duration> for Time<S> {
    #[inline]
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl<S: TimeScale> Sub<Time<S>> for Time<S> {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Time<S>) -> Self::Output {
        self.checked_elapsed(rhs)
            .expect("Time<S> - Time<S> overflowed i64 nanoseconds")
    }
}

impl Time<Glonass> {
    /// Constructor from GLONASS **day number** and **time of day** in second.
    ///
    /// GLONASS counts days from its epoch (1996-01-01 00:00:00 UTC(SU)) and
    /// uses seconds within the day.  Note: GLONASS time is UTC(SU) + 3 h;
    /// the internal representation here stores nanoseconds from the GLONASS
    /// epoch as received (i.e. UTC(SU) domain, +3 h already embedded).
    pub fn from_day_tod(day: u32, tod_s: f64) -> Result<Self, GnssTimeError> {
        if tod_s < 0.0 || tod_s >= 86_400.0 {
            return Err(GnssTimeError::InvalidInput("tod_s must be in [0, 86_400)"));
        }
        let day_nanos = (day as u64)
            .checked_mul(86_400_000_000_000)
            .ok_or(GnssTimeError::Overflow)?;
        let tod_nanos = (tod_s * 1_000_000_000.0) as u64;
        let total = day_nanos
            .checked_add(tod_nanos)
            .ok_or(GnssTimeError::Overflow)?;
        Ok(Time::from_nanos(total))
    }
}

impl Time<Gps> {
    /// Construct from a GPS **week number** and **time of week** in seconds.
    ///
    /// - `week`: GPS week number (0 = 1980-01-06, rolls over at 1024 without
    ///   rollover correction; this constructor accepts the raw value).
    /// - `tow_s`: time of week in seconds `[0, 604 800)`.
    ///
    pub fn from_week_tow(week: u16, tow_s: f64) -> Result<Self, GnssTimeError> {
        if tow_s < 0.0 || tow_s >= 604_800.0 {
            return Err(GnssTimeError::InvalidInput("tow_s must be in [0, 604_800]"));
        }
        let week_nanos = (week as u64)
            .checked_mul(604_800_000_000_000) // 604_800 s * 1e9
            .ok_or(GnssTimeError::Overflow)?;
        let tow_nanos = (tow_s * 1_000_000_000.0) as u64;
        let total = week_nanos
            .checked_add(tow_nanos)
            .ok_or(GnssTimeError::Overflow)?;

        Ok(Time::from_nanos(total))
    }

    /// GPS week number (integer division).
    #[inline]
    pub const fn week(self) -> u32 {
        (self.nanos / 604_800_000_000_000u64) as u32
    }

    /// Time of week in whole seconds.
    #[inline]
    pub const fn tow_seconds(self) -> u32 {
        ((self.nanos % 604_800_000_000_000u64) / 1_000_000_000u64) as u32
    }

    /// Sub-second nanosecond remainder.
    #[inline]
    pub const fn sub_second_nanos(self) -> u32 {
        (self.nanos % 1_000_000_000u64) as u32
    }
}

impl<S: TimeScale> PartialOrd for Time<S> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: TimeScale> Ord for Time<S> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.nanos.cmp(&other.nanos)
    }
}

impl<S: TimeScale> fmt::Debug for Time<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Time<{}>({}ns)", S::NAME, self.nanos)
    }
}

impl<S: TimeScale> fmt::Display for Time<S> {
    /// Formats as `"GPS +2300w 0s 0ns"` - scale name, day/week if GLONASS/GPS,
    /// then nanosecond and seconds remainder.
    ///
    /// For non-GPS scales a simple `"TAI +123456789s 0ns"` form is used.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.nanos / 1_000_000_000;
        let ns_rem = self.nanos % 1_000_000_000;

        write!(f, "{} +{}s {}ns", S::NAME, secs, ns_rem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{Beidou, Galileo, Glonass, Gps, Tai, Utc};
    #[allow(unused_imports)]
    use std::format;
    #[allow(unused_imports)]
    use std::string::ToString;

    #[test]
    fn test_size_equals_u64() {
        assert_eq!(core::mem::size_of::<Time<Glonass>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Gps>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Galileo>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Beidou>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Utc>>(), 8);
        assert_eq!(core::mem::size_of::<Time<Tai>>(), 8);
    }

    #[test]
    fn test_epoch_is_zero() {
        assert_eq!(Time::<Glonass>::EPOCH.as_nanos(), 0);
        assert_eq!(Time::<Gps>::EPOCH.as_nanos(), 0);
        assert_eq!(Time::<Galileo>::EPOCH.as_nanos(), 0);
        assert_eq!(Time::<Beidou>::EPOCH.as_nanos(), 0);
    }

    #[test]
    fn test_from_seconds_roundtrip() {
        let t = Time::<Gps>::from_seconds(86_400);

        assert_eq!(t.as_seconds(), 86_400);
        assert_eq!(t.as_nanos(), 86_400_000_000_000u64);
    }

    #[test]
    fn test_from_week_tow_zero() {
        let t = Time::<Gps>::from_week_tow(0, 0.0).unwrap();
        assert_eq!(t.as_nanos(), 0);
    }

    #[test]
    fn test_from_week_tow_roundtrip() {
        let t = Time::<Gps>::from_week_tow(2300, 3661.0).unwrap();
        assert_eq!(t.week(), 2300);
        assert_eq!(t.tow_seconds(), 3661);
    }

    #[test]
    fn test_from_week_tow_invalid_tow() {
        assert!(matches!(
            Time::<Gps>::from_week_tow(0, 604_800.0),
            Err(GnssTimeError::InvalidInput(_))
        ));
        assert!(matches!(
            Time::<Gps>::from_week_tow(0, -1.0),
            Err(GnssTimeError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_from_day_tod_zero() {
        let t = Time::<Glonass>::from_day_tod(0, 0.0).unwrap();
        assert_eq!(t.as_nanos(), 0);
    }

    #[test]
    fn test_from_day_tod_invalid() {
        assert!(matches!(
            Time::<Glonass>::from_day_tod(0, 86_400.0),
            Err(GnssTimeError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_add_positive_duration() {
        let t = Time::<Gps>::from_seconds(100);
        let d = Duration::from_seconds(50);
        assert_eq!((t + d).as_seconds(), 150);
    }

    #[test]
    fn test_add_negative_duration_moves_back() {
        let t = Time::<Gps>::from_seconds(100);
        let d = Duration::from_nanos(-50_000_000_000); // -50 s
        assert_eq!((t + d).as_seconds(), 50);
    }

    #[test]
    fn test_add_assign_duration() {
        let mut t = Time::<Gps>::from_seconds(10);
        t += Duration::from_seconds(5);
        assert_eq!(t.as_seconds(), 15);
    }

    #[test]
    fn test_sub_duration() {
        let t = Time::<Gps>::from_seconds(100);
        assert_eq!((t - Duration::from_seconds(30)).as_seconds(), 70);
    }

    #[test]
    fn test_sub_assign_duration() {
        let mut t = Time::<Gps>::from_seconds(100);
        t -= Duration::from_seconds(1);
        assert_eq!(t.as_seconds(), 99);
    }

    #[test]
    fn test_sub_times_gives_positive_duration() {
        let a = Time::<Gps>::from_seconds(200);
        let b = Time::<Gps>::from_seconds(100);
        assert_eq!((a - b).as_seconds(), 100);
    }

    #[test]
    fn test_sub_times_gives_negative_duration() {
        let a = Time::<Gps>::from_seconds(100);
        let b = Time::<Gps>::from_seconds(200);
        assert_eq!((a - b).as_seconds(), -100);
    }

    #[test]
    fn test_sub_same_instant_is_zero() {
        let t = Time::<Gps>::from_seconds(42);

        assert!((t - t).is_zero());
    }

    #[test]
    fn test_roundtrip_add_sub() {
        let t = Time::<Galileo>::from_seconds(1_000_000);
        let d = Duration::from_seconds(12_345);
        let t2 = t + d;
        let d2 = t2 - t;

        assert_eq!(d2, d);
        assert_eq!(t2 - d, t);
    }

    #[test]
    fn test_cross_scale_ops_are_impossible() {
        fn assert_no_cross<T: TimeScale>() {}

        // это компилируется ТОЛЬКО если нет смешанных impl
        assert_no_cross::<Gps>();
        assert_no_cross::<Glonass>();
    }

    #[test]
    #[should_panic]
    fn test_add_overflow_panics() {
        let t = Time::<Gps>::MAX;
        let d = Duration::from_nanos(1);
        let _ = t + d;
    }

    #[test]
    fn test_time_is_monotonic_with_duration() {
        let t = Time::<Gps>::from_seconds(100);
        let t2 = t + Duration::from_seconds(10);

        assert!(t2 > t);
    }

    #[test]
    fn test_time_never_underflows() {
        let t = Time::<Gps>::from_seconds(0);
        let d = Duration::from_seconds(10);

        let result = t.checked_sub_duration(d);
        assert!(result.is_none());
    }

    #[test]
    fn test_glonass_tod_bounds() {
        assert!(Time::<Glonass>::from_day_tod(0, 86_399.999).is_ok());
        assert!(Time::<Glonass>::from_day_tod(0, 86_400.0).is_err());
    }

    #[test]
    fn test_scale_identity_consistency() {
        assert_eq!(Gps::NAME, "GPS");
        assert_ne!(Gps::NAME, Glonass::NAME);
    }

    #[test]
    fn test_to_tai_fixed_scale() {
        let gps = Time::<Gps>::from_seconds(100);
        let tai = gps.to_tai().unwrap();

        // GPS -> TAI = +19s
        assert_eq!(tai.as_seconds(), 119);
    }

    #[test]
    fn test_from_tai_fixed_scale() {
        let tai = Time::<Tai>::from_seconds(119);
        let gps = Time::<Gps>::from_tai(tai).unwrap();

        assert_eq!(gps.as_seconds(), 100);
    }

    #[test]
    fn test_roundtrip_via_tai() {
        let original = Time::<Gps>::from_seconds(5000);
        let tai = original.to_tai().unwrap();
        let gps = Time::<Gps>::from_tai(tai).unwrap();

        assert_eq!(original, gps);
    }

    #[test]
    fn test_convert_between_fixed_scales() {
        let gps = Time::<Gps>::from_seconds(100);
        let bds = gps.try_convert::<Beidou>().unwrap();

        // GPS -> TAI (+19s) -> BDS (-5)
        // 100 + 19 - 5 = 114
        assert_eq!(bds.as_seconds(), 114);
    }

    #[test]
    fn test_convert_gps_to_galileo_identity() {
        let gps = Time::<Gps>::from_seconds(12345);
        let gal = gps.try_convert::<Galileo>().unwrap();

        // одинаковый offset -> одинаковое значение
        assert_eq!(gps.as_seconds(), gal.as_seconds());
    }

    #[test]
    fn test_to_tai_contextual_fails() {
        let glo = Time::<Glonass>::from_seconds(100);

        assert!(matches!(
            glo.to_tai(),
            Err(GnssTimeError::LeapSecondsRequired)
        ))
    }

    #[test]
    fn test_from_tai_contextual_fails() {
        let tai = Time::<Tai>::from_seconds(100);

        assert!(matches!(
            Time::<Utc>::from_tai(tai),
            Err(GnssTimeError::LeapSecondsRequired)
        ))
    }

    #[test]
    fn test_convert_to_contextual_fails() {
        let gps = Time::<Gps>::from_seconds(100);

        assert!(matches!(
            gps.try_convert::<Utc>(),
            Err(GnssTimeError::LeapSecondsRequired)
        ));
    }

    #[test]
    fn test_to_tai_overflow() {
        let t = Time::<Gps>::from_nanos(u64::MAX);
        let result = t.to_tai();

        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }

    #[test]
    fn test_from_tai_underflow() {
        let tai = Time::<Tai>::from_nanos(0);
        let result = Time::<Gps>::from_tai(tai);

        // 0 - 19s → < 0 → overflow
        assert!(matches!(result, Err(GnssTimeError::Overflow)));
    }
}
