//! Error types for the `gnss-time` crate.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GnssTimeError {
    Overflow,
    InvalidInput(&'static str),
    LeapSecondsRequired,
}

impl fmt::Display for GnssTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GnssTimeError::Overflow => {
                f.write_str("nanosecond arithmetic overflowed the i64 range")
            }
            GnssTimeError::InvalidInput(msg) => {
                write!(f, "invalid input: {msg}")
            }
            GnssTimeError::LeapSecondsRequired => f.write_str(
                "this conversion requires a LeapSeconds context (GLONASS <-> GPS or GPS <-> UTC)",
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for GnssTimeError {}
