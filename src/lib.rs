#![deny(unsafe_code)]
#![cfg_attr(not(test), no_std)]

pub mod driver;
pub mod error;
pub mod registers;

/// Crate-local `Result` type used throughout the MAX7219 driver.
///
/// This alias simplifies function signatures by defaulting the error type
/// to the crate's custom [`Error`] enum.
pub(crate) type Result<T> = core::result::Result<T, crate::error::Error>;

/// Maximum number of daisy-chained displays supported
pub const MAX_DISPLAYS: usize = 8;

/// Number of digits (0 to 7) controlled by one MAX7219
pub const NUM_DIGITS: u8 = 8;
