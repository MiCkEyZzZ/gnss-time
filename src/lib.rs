//! # gnss-time
//!
//! A type-safe GNSS time scale library with zero runtime overhead.
//!
//! Supports conversions between:
//! - GPS
//! - UTC
//! - GLONASS
//! - Galileo
//! - `BeiDou`
//! - TAI
//!
//! ## Feature flags
//!
//! | Feature  | Description                                                          |
//! |----------|----------------------------------------------------------------------|
//! | `std`    | `impl std::error::Error` for error types                             |
//! | `serde`  | `Serialize`/`Deserialize` for `Time<S>`, `Duration`, `DurationParts` |
//! | `defmt`  | `impl defmt::Format` for all public types                            |
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
//!
//! // ISO 8601 formatting
//! let dt = utc.to_civil();
//! println!("{dt}"); // e.g. "2023-01-01T00:00:18.000000000Z"
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::similar_names)]
#![warn(clippy::must_use_candidate)]
#![warn(clippy::missing_const_for_fn)]
#![warn(clippy::semicolon_if_nothing_returned)]

#[cfg(any(feature = "std", test))]
extern crate std;

////////////////////////////////////////////////////////////////////////////////
// Public modules
////////////////////////////////////////////////////////////////////////////////

// ISO 8601 civil date-time representation derived from `Time<Utc>`.
pub mod civil;

pub mod convert;
pub mod duration;
pub mod epoch;
pub mod error;
pub mod leap;
pub mod matrix;
pub mod scale;
pub mod time;

// Serde implementations for `Time<S>`, `Duration`, and `DurationParts`.
// Enabled by the `serde` feature flag.
#[cfg(feature = "serde")]
pub mod serde_impls;

////////////////////////////////////////////////////////////////////////////////
// Internal modules
////////////////////////////////////////////////////////////////////////////////

mod tables;

////////////////////////////////////////////////////////////////////////////////
// Public re-exports
////////////////////////////////////////////////////////////////////////////////

pub use civil::CivilDateTime;
pub use convert::*;
pub use duration::*;
pub use epoch::*;
pub use error::*;
pub use leap::*;
pub use matrix::*;
pub use scale::*;
pub use time::*;

////////////////////////////////////////////////////////////////////////////////
// Prelude
////////////////////////////////////////////////////////////////////////////////

// Common imports for typical usage.
pub mod prelude;
