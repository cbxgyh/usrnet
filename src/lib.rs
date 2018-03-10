#[cfg(test)]
#[macro_use]
extern crate assert_matches;
extern crate byteorder;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;

pub mod core;
pub mod examples;

#[cfg(target_os = "linux")]
pub mod linux;

use std::io::Error as IOError;
use std::result::Result as StdResult;

#[derive(Debug)]
pub enum Error {
    /// Indicates an error where an address could not be resolved.
    Address,
    /// Indicates an error where a socket binding has already been assigned.
    InUse,
    /// Indicates an error where a buffer, device, etc. is full or empty.
    Exhausted,
    /// Indicates an error where a packet or frame is malformed.
    Malformed,
    /// Indicates an error where a checksum is invalid.
    Checksum,
    /// Indicates an error where the operation was not performed.
    NoOp,
    /// Indicates a generic IO error.
    IO(IOError),
}

impl From<IOError> for Error {
    fn from(err: IOError) -> Error {
        Error::IO(err)
    }
}

pub type Result<T> = StdResult<T, Error>;
