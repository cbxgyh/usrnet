use std;

use core::repr::{
    Ipv4,
    Mac,
};

#[derive(Debug)]
pub enum Error {
    /// Indicates a generic IO error.
    IO(std::io::Error),
    /// Indicates a miscellaneous error with a message.
    Unknown(&'static str),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IO(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A low level interface for sending frames across a link.
pub trait Link {
    /// Sends a frame across a link.
    fn send(&mut self, buffer: &[u8]) -> Result<()>;

    /// Reads a frame from the underlying hardware and returns the size of
    /// frame. You should ensure that the buffer has at least MTU bytes to
    /// avoid errors.
    fn recv(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Returns the [MTU](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
    /// of the underlying hardware.
    fn get_max_transmission_unit(&self) -> Result<usize>;
}

/// A Link extension with an IPv4 address.
pub trait Ipv4Link: Link {
    /// Returns the Ipv4 address associated with the link.
    fn get_ipv4_addr(&self) -> Result<Ipv4>;
}

/// An ethernet Link.
pub trait EthernetLink: Link {
    /// Returns the hardware address associated with the link.
    fn get_ethernet_addr(&self) -> Result<Mac>;
}
