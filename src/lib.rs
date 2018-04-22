#[cfg(test)]
#[macro_use]
extern crate assert_matches;
extern crate byteorder;
extern crate get_if_addrs;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate rand;

pub mod core;
pub mod examples;

#[cfg(target_os = "linux")]
pub mod linux;

use std::io::Error as IOError;
use std::result::Result as StdResult;

use core::repr::Ipv4Address;
use core::socket::SocketAddr;

#[derive(Debug)]
pub enum Error {
    /// Indicates an error where a MAC address could not be resolved for an IPV4
    /// address.
    MacResolution(Ipv4Address),
    /// Indicates an error where a socket binding has already been assigned.
    BindingInUse(SocketAddr),
    /// Indicates an error where a socket buffer is full or empty, depending on the
    /// operation being performed.
    Exhausted,
    /// Indicates an error where a an incoming packet was ignored.
    Ignored,
    /// Indicates an error with a device/interface. This includes situations such as
    /// writes to a busy device or attempting reads on a device with no Ethernet frames.
    Device(Option<IOError>),
    /// Indicates an error where a packet or frame is malformed.
    Malformed,
    /// Indicates an error where a checksum is invalid.
    Checksum,
}

pub type Result<T> = StdResult<T, Error>;
