//! Error types for the `gnss-time` crate.

use core::fmt;

/// All errors that `gnss-time` operations can produce.
///
/// This type is marked `#[must_use]`: ignoring an error from a fallible
/// conversion silently discards the failure reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use = "errors must be handled; use `?` or `match` to inspect the failure"]
#[non_exhaustive]
pub enum GnssTimeError {
    /// Integer arithmetic overflowed the `u64` nanosecond range.
    Overflow,

    /// A coordinate or parameter was outside its valid domain.
    ///
    /// Carries a static description of which parameter was invalid.
    InvalidInput(&'static str),

    /// A conversion required leap-second context that was not provided.
    LeapSecondsRequired,
}

impl fmt::Display for GnssTimeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            GnssTimeError::Overflow => {
                f.write_str("nanosecond arithmetic overflowed the u64 range")
            }
            GnssTimeError::InvalidInput(msg) => {
                write!(f, "invalid input: {msg}")
            }
            GnssTimeError::LeapSecondsRequired => f.write_str(
                "this conversion requires a LeapSeconds context \
                 (GLONASS <-> GPS or GPS <-> UTC)",
            ),
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
                defmt::write!(f, "nanosecond arithmetic overflowed the u64 range")
            }
            GnssTimeError::InvalidInput(msg) => {
                defmt::write!(f, "invalid input: {}", msg)
            }
            GnssTimeError::LeapSecondsRequired => {
                defmt::write!(f, "conversion requires LeapSeconds context")
            }
        }
    }
}
