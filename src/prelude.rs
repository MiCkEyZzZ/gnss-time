//! Prelude: convenient imports for most common use cases.
//!
//! ```rust
//! use gnss_time::prelude::*;
//!
//! let gps = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
//! let tai: Time<Tai> = gps.into_scale().unwrap();
//! ```

pub use crate::{
    // Conversion traits
    convert::{IntoScale, IntoScaleWith},
    // Leap seconds
    leap::LeapSeconds,
    // Time scales
    scale::{Beidou, Galileo, Glonass, Gps, Tai, Utc},
    // Core types
    Duration,
    GnssTimeError,
    Time,
};
