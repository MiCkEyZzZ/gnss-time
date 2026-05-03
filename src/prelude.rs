//! Prelude: convenient imports for most use cases.
//!
//! ```rust
//! use gnss_time::{prelude::*, DurationParts};
//!
//! let gps = Time::<Gps>::from_week_tow(
//!     2345,
//!     DurationParts {
//!         seconds: 432_000,
//!         nanos: 0,
//!     },
//! )
//! .unwrap();
//! let tai: Time<Tai> = gps.into_scale().unwrap();
//! ```

pub use crate::{
    // Conversion traits
    convert::{ConvertResult, IntoScale, IntoScaleWith},
    // Leap seconds — static table
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
