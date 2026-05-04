//! # gnss-time
//!
//! A type-safe GNSS time scale library with zero runtime overhead.
//!
//! Supports conversions between:
//! - GPS
//! - UTC
//! - GLONASS
//! - Galileo
//! - BeiDou
//! - TAI
//!
//! Leap seconds are handled explicitly via a provider trait, ensuring
//! deterministic behavior and full `no_std` compatibility.
//!
//! ## Design goals
//!
//! - Zero allocations
//! - No global mutable state
//! - Fully deterministic conversions
//! - Embedded-friendly (`no_std`)
//! - Strong type safety across time scales
//!
//! ## Quick start
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
//! assert_eq!(gps.to_string(), "GPS 2345:432000.000");
//!
//! let utc: Time<Utc> = gps.into_scale_with(LeapSeconds::builtin()).unwrap();
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::must_use_candidate)]

#[cfg(any(feature = "std", test))]
extern crate std;

// Core modules
pub mod convert;
pub mod duration;
pub mod epoch;
pub mod error;
pub mod leap;
pub mod matrix;
pub mod scale;
pub mod time;

// Internal implementation details (not public API)
mod tables;

// Re-exports (public API surface)
pub use convert::*;
pub use duration::*;
pub use epoch::*;
pub use error::*;
pub use leap::*;
pub use matrix::*;
pub use scale::*;
pub use time::*;

/// Common imports for typical usage.
///
/// # Example
///
/// ```rust
/// use gnss_time::prelude::*;
/// ```
pub mod prelude;
