#![feature(proc_macro)]

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
extern crate byteorder;
extern crate libc;
#[macro_use]
extern crate log;

pub mod core;

#[cfg(target_os = "linux")]
pub mod linux;

#[derive(Debug)]
pub enum Error {
    /// Indicates an error where an address could not be resolved.
    Address,
    /// Indicates an error where a buffer, device, etc. is full or empty.
    Exhausted,
    /// Indicates an error where a packet or frame is malformed.
    Malformed,
    /// Indicates an error where a checksum is invalid.
    Checksum,
    /// Indicates an error where the operation was not performed.
    NoOp,
    /// Indicates a generic IO error.
    IO(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
