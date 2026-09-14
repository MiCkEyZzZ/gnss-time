//! # Static data tables
//!
//! Compile-time constant tables that back the crate's time-scale
//! conversions.
//!
//! ## Content
//!
//! - [`leap_seconds`] — the TAI–UTC leap second table (source: IERS Bulletin
//!   C), together with the [`LeapEntry`] type used to derive per-scale offsets.
//!
//! ## Re-exports
//!
//! Tables are re-exported at the module root for convenience
//! (`pub use leap_seconds::*`).

pub mod leap_seconds;

pub use leap_seconds::*;
