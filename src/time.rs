//! # `Time<S>` — основной тип временной метки.
//!
//! Хранит **наносекунды с эпохи шкалы `S`** в `u64`.
//! Фантомный параметр `S: TimeScale` обеспечивает корректность домена на этапе
//! компиляции — нельзя вычесть GPS-время из GLONASS-времени.
//!
//! ## Гарантия размера
//!
//! ```rust
//! # use gnss_time::{Time, scale::Gps};
//! assert_eq!(core::mem::size_of::<Time<Gps>>(), 8); // идентично u64
//! ```

use core::{
    fmt,
    marker::PhantomData,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use crate::{DisplayStyle, Duration, Glonass, GnssTimeError, Gps, OffsetToTai, Tai, TimeScale};

/// Временная метка в шкале времени `S`, хранимая как наносекунды от эпохи
/// шкалы.
///
/// # Примеры
///
/// ```rust
/// use gnss_time::{
///     scale::{Glonass, Gps},
///     Duration, Time,
/// };
///
/// let t: Time<Gps> = Time::from_nanos(0); // эпоха GPS
/// let later = t + Duration::from_seconds(3600);
/// assert_eq!((later - t).as_seconds(), 3600);
///
/// // Ошибка компиляции — разные шкалы несовместимы:
/// // let glo: Time<Glonass> = Time::from_nanos(0);
/// // let _ = later - glo; // ← ОШИБКА
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Time<S: TimeScale> {
    nanos: u64,
    _scale: PhantomData<S>,
}

impl<S: TimeScale> Time<S> {
    /// The scale's epoch - 0 nanoseconds.
    pub const EPOCH: Self = Time {
        nanos: 0,
        _scale: PhantomData,
    };

    /// Maximum representable instant (~584 years from epoch).
    pub const MAX: Self = Time {
        nanos: u64::MAX,
        _scale: PhantomData,
    };

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
        Time::from_nanos(secs * 1_000_000_000)
    }

    /// Construct from whole seconds, returning `None` on overflow.
    #[inline]
    pub const fn checked_from_seconds(secs: u64) -> Option<Self> {
        match secs.checked_mul(1_000_000_000) {
            Some(n) => Some(Time::from_nanos(n)),
            None => None,
        }
    }
}

impl<S: TimeScale> Time<S> {
    /// Конвертация в TAI с использованием фиксированного смещения шкалы.
    ///
    /// Возвращает [`GnssTimeError::LeapSecondsRequired`] для контекстных шкал
    /// (UTC, GLONASS) и [`GnssTimeError::Overflow`] при переполнении.
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

    /// Создание `Time<S>` из TAI при использовании фиксированного смещения.
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

    /// Прямая конвертация между шкалами с фиксированным смещением через TAI.
    pub fn try_convert<T: TimeScale>(self) -> Result<Time<T>, GnssTimeError> {
        let tai = self.to_tai()?;

        Time::<T>::from_tai(tai)
    }
}

impl<S: TimeScale> Time<S> {
    /// Сырые наносекунды от эпохи шкалы.
    #[inline(always)]
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    /// Целые секунды от эпохи (усечение).
    #[inline]
    pub const fn as_seconds(self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Секунды как `f64`. Возможна потеря точности для больших значений.
    #[inline]
    pub const fn as_seconds_f64(self) -> f64 {
        self.nanos as f64 / 1_000_000_000.0
    }
}

impl<S: TimeScale> Time<S> {
    /// Добавление `Duration`, `None` при переполнении/подпереполнении.
    #[inline]
    pub fn checked_add(
        self,
        d: Duration,
    ) -> Option<Self> {
        let result = (self.nanos as i128) + (d.as_nanos() as i128);

        if result < 0 || result > u64::MAX as i128 {
            return None;
        };

        Some(Time::from_nanos(result as u64))
    }

    /// Вычитание `Duration`, `None` при ошибке диапазона.
    #[inline]
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

    /// Saturating добавление (ограничение на EPOCH и MAX).
    #[inline]
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

    /// Saturating вычитание.
    #[inline]
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

    /// Fallible add — ошибка [`GnssTimeError::Overflow`] при неудаче.
    #[inline]
    pub fn try_add(
        self,
        d: Duration,
    ) -> Result<Self, GnssTimeError> {
        self.checked_add(d).ok_or(GnssTimeError::Overflow)
    }

    /// Fallible sub — ошибка [`GnssTimeError::Overflow`] при неудаче.
    #[inline]
    pub fn try_sub_duration(
        self,
        d: Duration,
    ) -> Result<Self, GnssTimeError> {
        self.checked_sub_duration(d).ok_or(GnssTimeError::Overflow)
    }
}

impl<S: TimeScale> Time<S> {
    /// Интервал `self - earlier`. `None`, если не помещается в `i64`.
    #[inline]
    pub const fn checked_elapsed(
        self,
        ealier: Time<S>,
    ) -> Option<Duration> {
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

impl Time<Glonass> {
    /// Construct from GLONASS **day number** and **time-of-day** in seconds.
    ///
    /// - `day`: days since GLONASS epoch (1996-01-01 00:00:00 UTC(SU)).
    /// - `tod_s`: time of day in seconds, must be in `[0, 86 400)`.
    ///
    /// # Errors
    ///
    /// [`GnssTimeError::InvalidInput`] if `tod_s ∉ [0, 86 400)`.
    pub fn from_day_tod(
        day: u32,
        tod_s: f64,
    ) -> Result<Self, GnssTimeError> {
        if !(0.0..86_400.0).contains(&tod_s) {
            return Err(GnssTimeError::InvalidInput("tod_s must be in [0, 86_400)"));
        }
        let day_ns = (day as u64)
            .checked_mul(86_400_000_000_000)
            .ok_or(GnssTimeError::Overflow)?;
        let tod_ns = (tod_s * 1_000_000_000.0) as u64;
        let total = day_ns.checked_add(tod_ns).ok_or(GnssTimeError::Overflow)?;

        Ok(Time::from_nanos(total))
    }

    /// Номер дня от эпохи GLONASS.
    #[inline]
    pub const fn day(self) -> u32 {
        (self.nanos / 86_400_000_000_000u64) as u32
    }

    /// Секунды времени суток.
    #[inline]
    pub const fn tod_seconds(self) -> u32 {
        ((self.nanos % 86_400_000_000_000u64) / 1_000_000_000u64) as u32
    }
}

impl Time<Gps> {
    /// Construct from a GPS **week number** and **time of week** in seconds.
    ///
    /// - `week`: GPS week number (0 = 1980-01-06, rolls over at 1024 without
    ///   rollover correction; this constructor accepts the raw value).
    /// - `tow_s`: time of week in seconds `[0, 604 800)`.
    ///
    /// # Errors
    ///
    /// [`GnssTimeError::InvalidInput`] if `tow_s ∉ [0, 604 800)`.
    /// [`GnssTimeError::Overflow`] if the result exceeds `u64::MAX` ns.
    ///
    /// # Example
    ///
    /// ```rust
    /// use gnss_time::{scale::Gps, Time};
    ///
    /// let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
    /// assert_eq!(t.week(), 2345);
    /// assert_eq!(t.tow_seconds(), 432_000);
    /// ```
    pub fn from_week_tow(
        week: u16,
        tow_s: f64,
    ) -> Result<Self, GnssTimeError> {
        if !(0.0..604_800.0).contains(&tow_s) {
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

    /// Sub-second nanosecond remainder within the current second.
    #[inline]
    pub const fn sub_second_nanos(self) -> u32 {
        (self.nanos % 1_000_000_000u64) as u32
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
    /// Format depends on the scale's [`DisplayStyle`]:
    ///
    /// | Style     | Example                   |
    /// |-----------|---------------------------|
    /// | `WeekTow` | `"GPS 2345:432000.000"`   |
    /// | `DayTod`  | `"GLO 10512:43200.000"`   |
    /// | `Simple`  | `"TAI +1000000000s 0ns"`  |
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
    }

    #[test]
    fn test_from_week_tow_zero() {
        let t = Time::<Gps>::from_week_tow(0, 0.0).unwrap();

        assert_eq!(t, Time::<Gps>::EPOCH);
    }

    #[test]
    fn test_from_week_tow_roundtrip() {
        let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();

        assert_eq!(t.week(), 2345);
        assert_eq!(t.tow_seconds(), 432_000);
        assert_eq!(t.sub_second_nanos(), 0);
    }

    #[test]
    fn test_from_week_tow_with_fractional() {
        let t = Time::<Gps>::from_week_tow(2300, 3661.5).unwrap();

        assert_eq!(t.week(), 2300);
        assert_eq!(t.tow_seconds(), 3661);
        assert_eq!(t.sub_second_nanos(), 500_000_000);
    }

    #[test]
    fn test_from_week_tow_invalid() {
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

        assert_eq!(t, Time::<Glonass>::EPOCH);
    }

    #[test]
    fn test_from_day_tod_roundtrip() {
        let t = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();

        assert_eq!(t.day(), 10_512);
        assert_eq!(t.tod_seconds(), 43_200);
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
        // Same offset → same TAI value → converting GPS→GAL keeps nanos
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
        // TAI(0) - 19s offset → negative GPS value → overflow
        let tai = Time::<Tai>::from_nanos(0);

        assert!(matches!(
            Time::<Gps>::from_tai(tai),
            Err(GnssTimeError::Overflow)
        ));
    }

    #[test]
    fn test_gps_display_week_tow_format() {
        // GPS 2345:432000.000 — the canonical example from the issue
        let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();

        assert_eq!(t.to_string(), "GPS 2345:432000.000");
    }

    #[test]
    fn test_gps_display_epoch_is_week_0() {
        let s = Time::<Gps>::EPOCH.to_string();

        assert_eq!(s, "GPS 0:000000.000");
    }

    #[test]
    fn test_gps_display_tow_zero_padded() {
        // TOW = 1 second → must show 000001
        let t = Time::<Gps>::from_week_tow(1, 1.0).unwrap();

        assert_eq!(t.to_string(), "GPS 1:000001.000");
    }

    #[test]
    fn test_gps_display_with_millis() {
        let t = Time::<Gps>::from_week_tow(100, 0.5).unwrap();

        assert_eq!(t.to_string(), "GPS 100:000000.500");
    }

    #[test]
    fn test_glonass_display_day_tod_format() {
        let t = Time::<Glonass>::from_day_tod(10_512, 43_200.0).unwrap();

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
        let t = Time::<Glonass>::from_day_tod(42, 3600.0).unwrap();

        assert_eq!(t.day(), 42);
        assert_eq!(t.tod_seconds(), 3600);
    }
}
