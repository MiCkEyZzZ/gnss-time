macro_rules! define_scale {
    (
        $(#[$meta:meta])*
        $name:ident,
        display = $display:literal,
        offset_to_tai = $offset:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $name;

        impl private::Sealed for $name {}

        impl TimeScale for $name {
            const NAME: &'static str = $display;
            const OFFSET_TO_TAI: OffsetToTai = $offset;
        }
    };
}

pub(crate) const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// Relation of a time scale to TAI (International Atomic Time).
///
/// This define how to convert *from this scale* into TAI:
///
/// `T_tai = T_self + offset`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetToTai {
    /// Fixed, compile-time known offset (no leap seconds involved)
    Fixed(i64),

    /// Requires external context (e.g. leap seconds)
    Contextual,
}

// sealed pattern — prevents external impls
mod private {
    pub trait Sealed {}
}

/// Marker trait for GNSS / atomic time scales.
///
/// This trait is **sealed** and cannot be implemented outside the crate.
///
/// Each scale defines:
/// - [`NAME`] — human-readable short identifier
/// - [`OFFSET_TO_TAI`] — how to convert this scale into TAI
pub trait TimeScale: private::Sealed + Copy + Clone + Eq + PartialEq + core::fmt::Debug {
    /// Short ASCII name used in Display/debug
    const NAME: &'static str;

    /// Defines conversion into TAI:
    ///
    /// STRICT CONTRACT:
    ///     T_tai = T_self + offset
    ///
    /// This must be consistent for all scales.
    /// Violating this breaks cross-scale conversions.
    ///
    /// For contextual scales (UTC, GLONASS),
    /// this requires external leap-second data.
    const OFFSET_TO_TAI: OffsetToTai;
}

define_scale!(
    /// GLONASS time (UTC(SU) + 3h internally, requires leap seconds)
    Glonass,
    display = "GLO",
    offset_to_tai = OffsetToTai::Contextual
);

define_scale!(
    /// GPS time scale
    ///
    /// GPS = TAI - 19s → TAI = GPS + 19s
    Gps,
    display = "GPS",
    offset_to_tai = OffsetToTai::Fixed(19 * NANOS_PER_SECOND)
);

define_scale!(
    /// Galileo System Time (GST)
    /// Aligned with GPS time (same offset to TAI)
    Galileo,
    display = "GAL",
    offset_to_tai = OffsetToTai::Fixed(19 * NANOS_PER_SECOND)
);

define_scale!(
    /// BeiDou time scale
    ///
    /// BDT = GPS + 14s → TAI = GPS + 19s → TAI = BDT + 5s
    Beidou,
    display = "BDS",
    offset_to_tai = OffsetToTai::Fixed(5 * NANOS_PER_SECOND)
);

define_scale!(
    /// International Atomic Time (TAI)
    Tai,
    display = "TAI",
    offset_to_tai = OffsetToTai::Fixed(0)
);

define_scale!(
    /// UTC (requires leap seconds)
    Utc,
    display = "UTC",
    offset_to_tai = OffsetToTai::Contextual
);

impl OffsetToTai {
    #[inline(always)]
    pub const fn fixed(self) -> Option<i64> {
        match self {
            OffsetToTai::Fixed(v) => Some(v),
            OffsetToTai::Contextual => None,
        }
    }

    #[inline(always)]
    pub const fn is_contextual(self) -> bool {
        matches!(self, OffsetToTai::Contextual)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::mem::size_of;

    use super::*;

    #[test]
    fn test_name_are_correct() {
        assert_eq!(Glonass::NAME, "GLO");
        assert_eq!(Gps::NAME, "GPS");
        assert_eq!(Galileo::NAME, "GAL");
        assert_eq!(Beidou::NAME, "BDS");
        assert_eq!(Tai::NAME, "TAI");
        assert_eq!(Utc::NAME, "UTC");
    }

    #[test]
    fn test_fixed_offsets() {
        assert_eq!(
            Gps::OFFSET_TO_TAI,
            OffsetToTai::Fixed(19 * NANOS_PER_SECOND)
        );
        assert_eq!(
            Galileo::OFFSET_TO_TAI,
            OffsetToTai::Fixed(19 * NANOS_PER_SECOND)
        );
        assert_eq!(
            Beidou::OFFSET_TO_TAI,
            OffsetToTai::Fixed(5 * NANOS_PER_SECOND)
        );
        assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
    }

    #[test]
    fn test_contextual_offsets() {
        assert_eq!(Utc::OFFSET_TO_TAI, OffsetToTai::Contextual);
        assert_eq!(Glonass::OFFSET_TO_TAI, OffsetToTai::Contextual);
    }

    #[test]
    fn test_scale_types_are_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<Glonass>();
        assert_copy::<Gps>();
        assert_copy::<Galileo>();
        assert_copy::<Beidou>();
        assert_copy::<Tai>();
        assert_copy::<Utc>();
    }

    #[test]
    fn test_tai_invariant_is_valid() {
        assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
        assert!(Tai::OFFSET_TO_TAI.fixed().unwrap() == 0);
    }

    #[test]
    fn test_fixed_scales_are_really_fixed() {
        let fixed_scales = [
            Gps::OFFSET_TO_TAI,
            Galileo::OFFSET_TO_TAI,
            Beidou::OFFSET_TO_TAI,
            Tai::OFFSET_TO_TAI,
        ];

        for scale in fixed_scales {
            assert!(scale.fixed().is_some(), "Expected Fixed, got Contextual");
        }
    }

    #[test]
    fn test_contextual_only_where_expected() {
        assert!(Utc::OFFSET_TO_TAI.is_contextual());
        assert!(Glonass::OFFSET_TO_TAI.is_contextual());
    }

    #[test]
    fn test_gps_and_galileo_are_aligned() {
        assert_eq!(Gps::OFFSET_TO_TAI, Galileo::OFFSET_TO_TAI);
    }

    #[test]
    fn test_names_are_unique() {
        let names = [
            Glonass::NAME,
            Gps::NAME,
            Galileo::NAME,
            Beidou::NAME,
            Tai::NAME,
            Utc::NAME,
        ];

        let set: HashSet<_> = names.iter().collect();

        assert_eq!(set.len(), names.len());
    }

    #[test]
    fn test_scale_is_zero_sized() {
        assert_eq!(size_of::<Glonass>(), 0);
        assert_eq!(size_of::<Gps>(), 0);
        assert_eq!(size_of::<Galileo>(), 0);
        assert_eq!(size_of::<Beidou>(), 0);
        assert_eq!(size_of::<Tai>(), 0);
        assert_eq!(size_of::<Utc>(), 0);
    }

    #[test]
    fn test_time_scales_follow_contract() {
        fn check<T: TimeScale>() {
            match T::OFFSET_TO_TAI {
                OffsetToTai::Fixed(v) => {
                    // Tai должна быть нейтральной точкой
                    if T::NAME == "TAI" {
                        assert_eq!(v, 0);
                    }
                }
                OffsetToTai::Contextual => {
                    // только UTC/GLONASS
                    assert!(T::NAME == "UTC" || T::NAME == "GLO");
                }
            }
        }

        check::<Glonass>();
        check::<Gps>();
        check::<Galileo>();
        check::<Beidou>();
        check::<Tai>();
        check::<Utc>();
    }
}
