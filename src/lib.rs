//! # gnss-time
//!
//! **Type-safe GNSS time scale with zero runtime overhead**.
//!
//! ## Quick start
//!
//! ```rust
//! use gnss_time::prelude::*;
//!
//! let gps = Time::<Gps>::from_week_tow(2345, 432_000.0).unwrap();
//! assert_eq!(gps.to_string(), "GPS 2345:432000.000");
//!
//! let utc: Time<Utc> = gps.into_scale_with(LeapSeconds::builtin()).unwrap();
//! ```

#![no_std]
#![forbid(unsafe_code)]
// #![deny(missing_docs)]

#[cfg(any(feature = "std", test))]
extern crate std;

// Основные модули
pub mod convert;
pub mod duration;
pub mod epoch;
pub mod error;
pub mod leap;
pub mod scale;
pub mod time;

// Внутренние таблицы (не публичные)
mod tables;

// Публичные реэкспорты
pub use convert::*;
pub use duration::*;
pub use epoch::*;
pub use error::*;
pub use leap::*;
pub use scale::*;
pub use time::*;

// Prelude для удобных импортов
pub mod prelude;
