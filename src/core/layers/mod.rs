pub mod arp;
pub mod ethernet;
pub mod icmpv4;
pub mod ipv4;

use std;

pub use self::arp::{
    hw_types as arp_hw_types,
    proto_types as arp_proto_types,
    Arp,
    Op as ArpOp,
};
pub use self::ethernet::{
    types as ethernet_types,
    Address as EthernetAddress,
    Frame as EthernetFrame,
};
pub use self::icmpv4::{
    Packet as Icmpv4Packet,
    Repr as Icmpv4Repr,
};
pub use self::ipv4::{
    Address as Ipv4Address,
    Packet as Ipv4Packet,
    flags as ipv4_flags,
    types as ipv4_types,
};

#[derive(Debug)]
pub enum Error {
    /// Indicates a size error with a buffer.
    Buffer,
    /// Indicates an encoding error.
    Encoding,
    /// Indicates a checksum error.
    Checksum,
}

pub type Result<T> = std::result::Result<T, Error>;
