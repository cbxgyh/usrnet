//! Serialization and deserialization of network packets.
//!
//! The `repr` module provides abstractions for serialization and deserializing
//! packets and frames at different network layers to/from byte buffers.

pub mod arp;
pub mod ethernet;
pub mod icmpv4;
pub mod ipv4;
pub mod udp;

pub use self::arp::{
    hw_types as arp_hw_types,
    proto_types as arp_proto_types,
    Arp,
    Op as ArpOp,
};
pub use self::ethernet::{
    eth_types,
    Address as EthernetAddress,
    Frame as EthernetFrame,
};
pub use self::icmpv4::{
    DestinationUnreachable as Icmpv4DestinationUnreachable,
    Packet as Icmpv4Packet,
    Repr as Icmpv4Repr,
    TimeExceeded as Icmpv4TimeExceeded,
};
pub use self::ipv4::{
    Address as Ipv4Address,
    AddressCidr as Ipv4AddressCidr,
    Packet as Ipv4Packet,
    Protocol as Ipv4Protocol,
    Repr as Ipv4Repr,
    flags as ipv4_flags,
    protocols as ipv4_protocols,
};
pub use self::udp::{
    Packet as UdpPacket,
    Repr as UdpRepr,
};
