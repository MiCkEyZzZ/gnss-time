//! # gnss-time
//!
//! **A type-safe GNSS time scale library with zero runtime overhead.**
//!
//! ## Quick start
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

// Internal tables (not public API)
mod tables;

// Public re-exports
pub use convert::*;
pub use duration::*;
pub use epoch::*;
pub use error::*;
pub use leap::*;
pub use matrix::*;
pub use scale::*;
pub use time::*;

// Prelude for convenient imports
pub mod prelude;
