//! # GNSS Time Scale Marker Types
//!
//! This module defines **compile-time marker types** for GNSS and atomic
//! time scales.
//!
//! Each GNSS system operates on its own time scale with a fixed or
//! contextual relationship to **TAI (International Atomic Time)**.
//!
//! ## Design principles
//!
//! - Each scale is a **zero-sized type (ZST)**
//! - No runtime state is stored
//! - All conversions are expressed through **TAI as a pivot scale**
//! - The trait [`TimeScale`] is **sealed** to prevent external implementations
//!
//! ## Sealed trait
//!
//! [`TimeScale`] cannot be implemented outside this crate.
//! This ensures:
//!
//! - correctness of offset invariants
//! - consistency of conversion rules
//! - prevention of user-defined incompatible scales
//!
//! ## Display formats
//!
//! | Scale   | Format                     |
//! |---------|----------------------------|
//! | GPS     | `GPS 2345:432000.000`      |
//! | GLONASS | `GLO 10512:43200.000`      |
//! | Galileo | `GAL 1303:432000.000`      |
//! | BeiDou  | `BDT 960:432000.000`       |
//! | TAI     | `TAI +1000000000s 0ns`     |
//! | UTC     | `UTC +1000000000s 0ns`     |

use crate::epoch::CivilDate;

/// Internal module implementing the sealed trait pattern.
mod private {
    pub trait Sealed {}
}

macro_rules! define_scale {
    (
        $(#[$meta:meta])*
        $name:ident,
        display  = $display:literal,
        offset   = $offset:expr,
        epoch    = $epoch:expr,
        style    = $style:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $name;

        impl private::Sealed for $name {}

        impl TimeScale for $name {
            const NAME:          &'static str  = $display;
            const OFFSET_TO_TAI: OffsetToTai   = $offset;
            const EPOCH_CIVIL:   CivilDate     = $epoch;
            const DISPLAY_STYLE: DisplayStyle  = $style;
        }
    };
}

pub(crate) const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// Relationship between a time scale and TAI.
///
/// The defining invariant is:
///
/// ```text
/// T_tai = T_scale + offset
/// ```
///
/// ## Variants
///
/// - `Fixed`: constant offset (no external dependencies)
/// - `Contextual`: depends on runtime leap-second information
///
/// Contextual scales require a leap-second provider for correct conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OffsetToTai {
    /// Constant offset relative to TAI.
    Fixed(i64),

    /// Offset depends on external context (e.g. leap seconds).
    Contextual,
}

/// Defines how a [`crate::Time<S>`] value is formatted for display.
///
/// This affects `Display` and debug output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayStyle {
    /// Week-based format:
    /// `NAME WWW:SSSSSS.mm`
    ///
    /// Used by GPS, Galileo, BeiDou.
    WeekTow,

    /// Day-based format:
    /// `NAME DDDDD:SSSSS.mmm`
    ///
    /// Used by GLONASS.
    DayTod,

    /// Simple scalar format:
    /// `NAME +Ss Nns`
    ///
    /// Used by TAI and UTC.
    Simple,
}

/// Marker trait for GNSS and atomic time scales.
///
/// This trait is **sealed** and cannot be implemented externally.
///
/// ## Contract
///
/// Each implementation MUST define:
///
/// - a unique name (`NAME`)
/// - a consistent offset to TAI (`OFFSET_TO_TAI`)
/// - a reference epoch (`EPOCH_CIVIL`)
/// - a display format (`DISPLAY_STYLE`)
pub trait TimeScale: private::Sealed + Copy + Clone + Eq + PartialEq + core::fmt::Debug {
    /// Short identifier used in formatting.
    const NAME: &'static str;

    /// Offset relative to TAI:
    ///
    /// ```text
    /// T_tai = T_self + offset
    /// ```
    const OFFSET_TO_TAI: OffsetToTai;

    /// Civil epoch of the scale (where time == 0).
    const EPOCH_CIVIL: CivilDate;

    /// Formatting style for display output.
    const DISPLAY_STYLE: DisplayStyle;
}

define_scale!(
    /// GLONASS — Russian time system (UTC(SU) + 3 hours)
    ///
    /// - Epoch: 1996-01-01 00:00:00 UTC(SU)
    /// - Operates relative to UTC(SU)
    /// - Requires leap-second handling
    /// - Format: `"GLO 10512:43200.000"`
    Glonass,
    display = "GLO",
    offset = OffsetToTai::Contextual,
    epoch   = CivilDate::new(1996, 1, 1),
    style   = DisplayStyle::DayTod
);

define_scale!(
    /// GPS — American Global Positioning System
    ///
    /// - Epoch: 1980-01-06 UTC
    /// - GPS = TAI − 19 seconds
    /// - No leap seconds (fixed offset)
    /// - Format: `"GPS 2345:432000.000"`
    Gps,
    display = "GPS",
    offset  = OffsetToTai::Fixed(19 * NANOS_PER_SECOND),
    epoch   = CivilDate::new(1980, 1, 6),
    style   = DisplayStyle::WeekTow
);

define_scale!(
    /// Galileo — European navigation system (GST)
    ///
    /// - Epoch: 1999-08-22 UTC
    /// - Same offset as GPS (TAI − 19 s)
    /// - Equal numeric values represent the same physical instant
    /// - Format: `"GAL 1303:432000.000"`
    Galileo,
    display = "GAL",
    offset = OffsetToTai::Fixed(19 * NANOS_PER_SECOND),
    epoch = CivilDate::new(1999, 8, 22),
    style   = DisplayStyle::WeekTow
);

define_scale!(
    /// BeiDou — Chinese navigation system (BDT)
    ///
    /// - Epoch: 2006-01-01 UTC
    /// - BDT = TAI − 33 seconds
    /// - BDT = GPS − 14 seconds
    /// - Format: `"BDT 960:432000.000"`
    Beidou,
    display = "BDT",
    offset = OffsetToTai::Fixed(33 * NANOS_PER_SECOND),
    epoch = CivilDate::new(2006, 1, 1),
    style = DisplayStyle::WeekTow
);

define_scale!(
    /// TAI — International Atomic Time
    ///
    /// - Epoch: 1958-01-01
    /// - Base scale for all conversions
    /// - TAI = TAI + 0
    /// - Format: `"TAI +Ss Nns"`
    ///
    /// # Important
    ///
    /// Inside this crate, TAI is used as the pivot for conversions,
    /// not as an absolute scale from 1958 onward (this is planned separately).
    Tai,
    display = "TAI",
    offset = OffsetToTai::Fixed(0),
    epoch = CivilDate::new(1958, 1, 1),
    style = DisplayStyle::Simple
);

define_scale!(
    /// UTC — Coordinated Universal Time
    ///
    /// - UTC = TAI − LS(t)
    /// - Requires a runtime leap-second table
    /// - Format: `"UTC +Ss Nns"`
    Utc,
    display = "UTC",
    offset = OffsetToTai::Contextual,
    epoch = CivilDate::new(1972, 1, 1),
    style = DisplayStyle::Simple
);

impl OffsetToTai {
    /// Returns the fixed offset in nanoseconds.
    #[inline(always)]
    #[must_use]
    pub const fn fixed(self) -> Option<i64> {
        match self {
            OffsetToTai::Fixed(v) => Some(v),
            OffsetToTai::Contextual => None,
        }
    }

    /// Returns `true` for scales that require runtime context (UTC, GLONASS).
    #[inline(always)]
    #[must_use]
    pub const fn is_contextual(self) -> bool {
        matches!(self, OffsetToTai::Contextual)
    }

    /// Returns `true` for scale with a fixed TAI offset.
    #[inline(always)]
    #[must_use]
    pub const fn is_fixed(self) -> bool {
        matches!(self, OffsetToTai::Fixed(_))
    }
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, mem::size_of};

    use super::*;

    #[test]
    fn test_name_are_correct() {
        assert_eq!(Glonass::NAME, "GLO");
        assert_eq!(Gps::NAME, "GPS");
        assert_eq!(Galileo::NAME, "GAL");
        assert_eq!(Beidou::NAME, "BDT");
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
            OffsetToTai::Fixed(33 * NANOS_PER_SECOND)
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
    fn test_gps_and_galileo_are_aligned() {
        // Same TAI offset → synchronous (time-aligned) instants
        assert_eq!(Gps::OFFSET_TO_TAI, Galileo::OFFSET_TO_TAI);
    }

    #[test]
    fn test_tai_invariant_is_valid() {
        assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
        assert!(Tai::OFFSET_TO_TAI.fixed().unwrap() == 0);
    }

    #[test]
    fn test_names_are_unique() {
        let names = [
            Gps::NAME,
            Glonass::NAME,
            Galileo::NAME,
            Beidou::NAME,
            Tai::NAME,
            Utc::NAME,
        ];
        let set: HashSet<_> = names.iter().collect();

        assert_eq!(set.len(), names.len());
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
    fn test_scale_is_zero_sized() {
        assert_eq!(size_of::<Glonass>(), 0);
        assert_eq!(size_of::<Gps>(), 0);
        assert_eq!(size_of::<Galileo>(), 0);
        assert_eq!(size_of::<Beidou>(), 0);
        assert_eq!(size_of::<Tai>(), 0);
        assert_eq!(size_of::<Utc>(), 0);
    }

    #[test]
    fn test_scale_is_copy() {
        fn assert_copy<T: Copy + Clone + Eq + PartialEq + core::fmt::Debug>() {}
        assert_copy::<Glonass>();
        assert_copy::<Gps>();
        assert_copy::<Galileo>();
        assert_copy::<Beidou>();
        assert_copy::<Tai>();
        assert_copy::<Utc>();
    }

    #[test]
    fn test_display_styles() {
        assert_eq!(Gps::DISPLAY_STYLE, DisplayStyle::WeekTow);
        assert_eq!(Glonass::DISPLAY_STYLE, DisplayStyle::DayTod);
        assert_eq!(Galileo::DISPLAY_STYLE, DisplayStyle::WeekTow);
        assert_eq!(Beidou::DISPLAY_STYLE, DisplayStyle::WeekTow);
        assert_eq!(Tai::DISPLAY_STYLE, DisplayStyle::Simple);
        assert_eq!(Utc::DISPLAY_STYLE, DisplayStyle::Simple);
    }

    #[test]
    fn test_offset_to_tai_helpers() {
        assert!(OffsetToTai::Fixed(0).is_fixed());
        assert!(!OffsetToTai::Fixed(0).is_contextual());
        assert!(OffsetToTai::Contextual.is_contextual());
        assert!(!OffsetToTai::Contextual.is_fixed());
        assert_eq!(OffsetToTai::Fixed(42).fixed(), Some(42));
        assert_eq!(OffsetToTai::Contextual.fixed(), None);
    }

    #[test]
    fn test_epoch_civil_dates() {
        assert_eq!(Gps::EPOCH_CIVIL.year, 1980);
        assert_eq!(Glonass::EPOCH_CIVIL.year, 1996);
        assert_eq!(Galileo::EPOCH_CIVIL.year, 1999);
        assert_eq!(Beidou::EPOCH_CIVIL.year, 2006);
        assert_eq!(Tai::EPOCH_CIVIL.year, 1958);
    }

    #[test]
    fn test_tai_invariant() {
        assert_eq!(Tai::OFFSET_TO_TAI, OffsetToTai::Fixed(0));
        assert_eq!(Tai::OFFSET_TO_TAI.fixed(), Some(0));
    }

    #[test]
    fn test_contract_all_scales() {
        fn check<T: TimeScale>() {
            match T::OFFSET_TO_TAI {
                OffsetToTai::Fixed(0) => assert_eq!(T::NAME, "TAI"),
                OffsetToTai::Fixed(_) => { /* GPS, GAL, BDT */ }
                OffsetToTai::Contextual => {
                    assert!(T::NAME == "UTC" || T::NAME == "GLO")
                }
            }
        }
        check::<Gps>();
        check::<Glonass>();
        check::<Galileo>();
        check::<Beidou>();
        check::<Tai>();
        check::<Utc>();
    }
}
