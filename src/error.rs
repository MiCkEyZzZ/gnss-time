//! Error types for the `gnss-time` crate.
//!
//! This module defines the unified error type used across all fallible
//! operations in the crate, including conversions, arithmetic, and
//! time-scale transformations.
//!
//! The design follows a strict principle:
//!
//! - **No hidden failure modes** — all fallible operations return `Result`
//! - **Explicit error context** — each variant describes a recoverable class of
//!   failure
//! - **`#[non_exhaustive]` for forward compatibility**

use core::fmt;

/// Errors returned by fallible `gnss-time` operations.
///
/// `GnssTimeError` is used throughout the crate for arithmetic overflow,
/// invalid inputs, and missing auxiliary data (e.g. leap seconds).
///
/// This type is intentionally `#[non_exhaustive]` to allow new error cases
/// without breaking semver compatibility.
///
/// # Usage
///
/// ```rust
/// use gnss_time::GnssTimeError;
///
/// fn example() -> Result<(), GnssTimeError> {
///     Err(GnssTimeError::Overflow)
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use = "errors must be handled; use `?` or `match` to inspect the failure"]
#[non_exhaustive]
pub enum GnssTimeError {
    /// Arithmetic overflow occurred during nanosecond-based computations.
    ///
    /// This indicates that an operation exceeded the representable range of
    /// the underlying `i64` nanosecond storage.
    Overflow,

    /// The provided input value is invalid for the requested operation.
    ///
    /// The attached string provides a short static description of the issue.
    InvalidInput(&'static str),

    /// The operation requires leap-second information that is not available.
    ///
    /// This is typically required for conversions between UTC-based and
    /// atomic time scales (e.g. GPS ↔ UTC, GLONASS ↔ GPS).
    LeapSecondsRequired,
}

impl fmt::Display for GnssTimeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            GnssTimeError::Overflow => f.write_str("arithmetic overflow in nanosecond computation"),
            GnssTimeError::InvalidInput(msg) => {
                write!(f, "invalid input: {msg}")
            }
            GnssTimeError::LeapSecondsRequired => {
                f.write_str("leap-second data required for this conversion")
            }
        }
    }
}

// `std::error::Error` impl behind the `std` feature gate.
#[cfg(feature = "std")]
impl std::error::Error for GnssTimeError {}

// defmt support: embedded logging via probe-rs / defmt-rtt.
#[cfg(feature = "defmt")]
impl defmt::Format for GnssTimeError {
    fn format(
        &self,
        f: defmt::Formatter,
    ) {
        match self {
            GnssTimeError::Overflow => {
                defmt::write!(f, "arithmetic overflow in nanoseconds")
            }
            GnssTimeError::InvalidInput(msg) => {
                defmt::write!(f, "invalid input: {}", msg)
            }
            GnssTimeError::LeapSecondsRequired => {
                defmt::write!(f, "leap-second data required")
            }
        }
    }
}
