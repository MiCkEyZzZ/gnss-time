//! Prelude: commonly used imports.
//!
//! This module re-exports the most frequently used types and functions
//! to simplify typical GNSS time conversion workflows.
//!
//! ## Example
//!
//! ```rust
//! use gnss_time::prelude::*;
//!
//! let gps = Time::<Gps>::from_week_tow(
//!     2345,
//!     DurationParts {
//!         seconds: 432_000,
//!         nanos: 0,
//!     },
//! )
//! .unwrap();
//!
//! let tai: Time<Tai> = gps.into_scale().unwrap();
//! ```

pub use crate::{
    civil::CivilDateTime,

    // Conversion traits
    convert::{ConvertResult, IntoScale, IntoScaleWith},

    // Unix time epoch offsets
    epoch::{GPS_EPOCH_UNIX_S, UTC_EPOCH_UNIX_OFFSET_NS, UTC_EPOCH_UNIX_OFFSET_S},

    // Leap seconds — static and runtime tables
    leap::{
        gps_to_utc, utc_to_gps, LeapEntry, LeapExtendError, LeapSeconds, LeapSecondsProvider,
        RuntimeLeapSeconds, RUNTIME_CAPACITY,
    },

    // Time scales
    scale::{Beidou, Galileo, Glonass, Gps, Tai, Utc},

    // Core types
    Duration,
    DurationParts,
    GnssTimeError,
    Time,
};
