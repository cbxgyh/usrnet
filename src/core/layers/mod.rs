pub mod arp;
pub mod ethernet;
pub mod ipv4;

use std;

pub use self::arp::{
    Arp,
    HwType as ArpHwType,
    Op as ArpOp,
    ProtoType as ArpProtoType,
};
pub use self::ethernet::{
    types as ethernet_types,
    Address as EthernetAddress,
    Frame as EthernetFrame,
};
pub use self::ipv4::{
    Address as Ipv4Address,
    Packet as Ipv4Packet,
};

#[derive(Debug)]
pub enum Error {
    /// Indicates a size error with a buffer.
    Buffer,
    /// Indicates an encoding error.
    Encoding,
}

pub type Result<T> = std::result::Result<T, Error>;
