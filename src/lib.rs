#![no_std]
// #![deny(missing_docs)]
#![deny(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

// Allow std in test builds (cargo test always links std).
#[cfg(test)]
extern crate std;

pub mod duration;
pub mod error;
pub mod scale;
pub mod time;

pub use duration::*;
pub use error::*;
pub use scale::*;
pub use time::*;
