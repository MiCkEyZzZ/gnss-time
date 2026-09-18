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

use core::fmt::{self};

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

    /// A string could not be parsed into the requested type.
    ParseError(&'static str),

    /// The operation requires leap-second information that is not available.
    ///
    /// This is typically required for conversions between UTC-based and
    /// atomic time scales (e.g. GPS ↔ UTC, GLONASS ↔ GPS).
    LeapSecondsRequired,

    /// The value lies outside the representable range of the timestamp.
    ///
    /// This occurs when a Unix timestamp is earlier than the UTC epoch
    /// (1972-01-01) or when a conversion would result in a negative
    /// nanosecond count.
    OutOfRange,
}

impl fmt::Display for GnssTimeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            GnssTimeError::Overflow => f.write_str("arithmetic overflow in nanoseconds"),
            GnssTimeError::InvalidInput(msg) => {
                write!(f, "invalid input: {msg}")
            }
            GnssTimeError::LeapSecondsRequired => f.write_str("leap-second data required"),
            GnssTimeError::OutOfRange => f.write_str("timestamp is out of representable range"),
            GnssTimeError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

// `std::error::Error` impl behind the `std` feature gate.
#[cfg(feature = "std")]
impl std::error::Error for GnssTimeError {}

// defmt support: embedded logging via probe-rs / defmt-rtt.
#[cfg(feature = "defmt")]
#[allow(clippy::match_same_arms)]
impl defmt::Format for GnssTimeError {
    fn format(
        &self,
        f: defmt::Formatter,
    ) {
        match self {
            GnssTimeError::Overflow => {
                defmt::write!(f, "arithmetic overflow in nanoseconds");
            }
            GnssTimeError::InvalidInput(msg) => {
                defmt::write!(f, "invalid input: {}", msg);
            }
            GnssTimeError::LeapSecondsRequired => {
                defmt::write!(f, "leap-second data required");
            }
            GnssTimeError::OutOfRange => {
                defmt::write!(f, "timestamp is out of representable range");
            }
            GnssTimeError::ParseError(msg) => defmt::write!(f, "parse error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::*;

    #[test]
    fn test_overflow_display() {
        let error = GnssTimeError::Overflow;

        assert_eq!(error.to_string(), "arithmetic overflow in nanoseconds");
    }

    #[test]
    fn test_invalid_input_display() {
        let error = GnssTimeError::InvalidInput("invalid scale");

        assert_eq!(error.to_string(), "invalid input: invalid scale");
    }

    #[test]
    fn test_parse_error_display() {
        let error = GnssTimeError::ParseError("invalid RFC 3339 timestamp");

        assert_eq!(error.to_string(), "parse error: invalid RFC 3339 timestamp");
    }

    #[test]
    fn test_leap_seconds_required_display() {
        let error = GnssTimeError::LeapSecondsRequired;

        assert_eq!(error.to_string(), "leap-second data required");
    }

    #[test]
    fn test_out_of_range_display() {
        let error = GnssTimeError::OutOfRange;

        assert_eq!(error.to_string(), "timestamp is out of representable range");
    }

    #[test]
    fn test_invalid_input_preserves_message() {
        let message = "custom validation failure";
        let error = GnssTimeError::InvalidInput(message);

        assert_eq!(error, GnssTimeError::InvalidInput(message));
        assert_eq!(
            error.to_string(),
            "invalid input: custom validation failure"
        );
    }

    #[test]
    fn test_parse_error_preserves_message() {
        let message = "invalid integer";
        let error = GnssTimeError::ParseError(message);

        assert_eq!(error, GnssTimeError::ParseError(message));
        assert_eq!(error.to_string(), "parse error: invalid integer");
    }

    #[test]
    fn test_empty_messages_are_supported() {
        assert_eq!(
            GnssTimeError::InvalidInput("").to_string(),
            "invalid input: "
        );
        assert_eq!(GnssTimeError::ParseError("").to_string(), "parse error: ");
    }

    #[test]
    fn test_messages_with_special_characters_are_preserved() {
        let invalid = GnssTimeError::InvalidInput("expected week/TOW: got 0/NaN");
        assert_eq!(
            invalid.to_string(),
            "invalid input: expected week/TOW: got 0/NaN"
        );

        let parse = GnssTimeError::ParseError("expected YYYY-MM-DDTHH:MM:SS");

        assert_eq!(
            parse.to_string(),
            "parse error: expected YYYY-MM-DDTHH:MM:SS"
        );
    }

    #[test]
    fn test_all_variants_are_distinct() {
        let errors = [
            GnssTimeError::Overflow,
            GnssTimeError::InvalidInput("x"),
            GnssTimeError::ParseError("x"),
            GnssTimeError::LeapSecondsRequired,
            GnssTimeError::OutOfRange,
        ];

        for (i, lhs) in errors.iter().enumerate() {
            for (j, rhs) in errors.iter().enumerate() {
                assert_eq!(lhs == rhs, i == j);
            }
        }
    }

    #[test]
    fn test_same_parameterized_variants_are_equal() {
        assert_eq!(
            GnssTimeError::InvalidInput("x"),
            GnssTimeError::InvalidInput("x")
        );
        assert_eq!(
            GnssTimeError::ParseError("x"),
            GnssTimeError::ParseError("x")
        );
    }

    #[test]
    fn test_different_parameterized_variants_are_not_equal() {
        assert_ne!(
            GnssTimeError::InvalidInput("x"),
            GnssTimeError::InvalidInput("y")
        );
        assert_ne!(
            GnssTimeError::ParseError("x"),
            GnssTimeError::ParseError("y")
        );
        assert_ne!(
            GnssTimeError::InvalidInput("x"),
            GnssTimeError::ParseError("x")
        );
    }

    #[test]
    fn test_clone_equals_original() {
        let errors = [
            GnssTimeError::Overflow,
            GnssTimeError::InvalidInput("invalid"),
            GnssTimeError::ParseError("parse"),
            GnssTimeError::LeapSecondsRequired,
            GnssTimeError::OutOfRange,
        ];

        for error in errors {
            assert_eq!(error, error.clone());
        }
    }

    #[test]
    fn test_copy_preserves_value() {
        let original = GnssTimeError::InvalidInput("invalid");
        let copied = original;

        assert_eq!(original, copied);
    }

    #[test]
    fn test_hash_is_consistent_for_equal_values() {
        use core::hash::{Hash, Hasher};

        #[derive(Default)]
        struct TestHasher(u64);

        impl Hasher for TestHasher {
            fn finish(&self) -> u64 {
                self.0
            }

            fn write(
                &mut self,
                bytes: &[u8],
            ) {
                for byte in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(u64::from(*byte));
                }
            }
        }

        let lhs = GnssTimeError::InvalidInput("invalid");
        let rhs = GnssTimeError::InvalidInput("invalid");
        let mut lhs_hasher = TestHasher::default();
        let mut rhs_hasher = TestHasher::default();

        lhs.hash(&mut lhs_hasher);
        rhs.hash(&mut rhs_hasher);

        assert_eq!(lhs_hasher.finish(), rhs_hasher.finish());
    }

    #[test]
    fn hash_distinguishes_different_values() {
        use core::hash::{Hash, Hasher};

        #[derive(Default)]
        struct TestHasher(u64);

        impl Hasher for TestHasher {
            fn finish(&self) -> u64 {
                self.0
            }

            fn write(
                &mut self,
                bytes: &[u8],
            ) {
                for byte in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(u64::from(*byte));
                }
            }
        }

        let lhs = GnssTimeError::InvalidInput("a");
        let rhs = GnssTimeError::InvalidInput("b");

        let mut lhs_hasher = TestHasher::default();
        let mut rhs_hasher = TestHasher::default();

        lhs.hash(&mut lhs_hasher);
        rhs.hash(&mut rhs_hasher);

        assert_ne!(lhs_hasher.finish(), rhs_hasher.finish());
    }

    #[test]
    fn errors_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<GnssTimeError>();
    }

    #[cfg(feature = "std")]
    #[test]
    fn implements_std_error() {
        fn assert_error<T: std::error::Error>() {}

        assert_error::<GnssTimeError>();

        let error = GnssTimeError::Overflow;
        let error: &dyn std::error::Error = &error;

        assert_eq!(error.to_string(), "arithmetic overflow in nanoseconds");
    }

    #[cfg(feature = "std")]
    #[test]
    fn std_error_has_no_source() {
        use std::error::Error;

        let errors = [
            GnssTimeError::Overflow,
            GnssTimeError::InvalidInput("invalid"),
            GnssTimeError::ParseError("parse"),
            GnssTimeError::LeapSecondsRequired,
            GnssTimeError::OutOfRange,
        ];

        for error in errors {
            assert!(error.source().is_none());
        }
    }

    #[cfg(feature = "defmt")]
    #[test]
    fn defmt_format_runs_for_all_variants() {
        use defmt::Format;

        let errors = [
            GnssTimeError::Overflow,
            GnssTimeError::InvalidInput("reason"),
            GnssTimeError::ParseError("token"),
            GnssTimeError::LeapSecondsRequired,
            GnssTimeError::OutOfRange,
        ];

        for error in errors {
            let formatter = defmt::export::make_formatter();
            error.format(formatter);
        }
    }
}
