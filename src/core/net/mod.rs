use std;

pub mod arp;

#[derive(Debug)]
pub enum Error {
    /// Indicates a buffer overflow.
    Overflow,
}

pub type Result<T> = std::result::Result<T, Error>;
