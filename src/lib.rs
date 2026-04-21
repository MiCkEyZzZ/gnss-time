//! # gnss-time
//!
//! **Type-safe GNSS time scale with zero runtime overhead**.
//!
//! ```rust
//! use gnss_time::{
//!     leap::{gps_to_utc, LeapSeconds},
//!     scale::Gps,
//!     Duration, Time,
//! };
//!
//! let t = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
//! assert_eq!(t.to_string(), "GPS 2345:432000.000");
//!
//! let utc = gps_to_utc(t, LeapSeconds::builtin()).unwrap();
//! let _ = utc; // Time<Utc>
//! ```
//!
//! ## `no_std` by default
//!
//! Enable the `std` feature for [`core::error::Error`] on [`GnssTimeError`].

#![no_std]
// #![deny(missing_docs)]
#![deny(unsafe_code)]

// Allow std in test builds (cargo test always links std).
#[cfg(any(feature = "std", test))]
extern crate std;

pub mod duration;
pub mod epoch;
pub mod error;
pub mod leap;
pub mod scale;
pub mod tables;
pub mod time;

pub use duration::*;
pub use epoch::*;
pub use error::*;
pub use leap::*;
pub use scale::*;
pub use tables::*;
pub use time::*;
