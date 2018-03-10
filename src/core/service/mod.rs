//! Packet processing services for different network layers.
//!
//! The `services` module deals with packet transmission and reception logic at
//! different layers of the network stack.

pub mod arp;
pub mod ethernet;
pub mod icmpv4;
pub mod ipv4;
pub mod socket;
pub mod udp;

use core::arp_cache::ArpCache;
use core::dev::Device;
use core::repr::{
    EthernetAddress,
    Ipv4Address,
    Ipv4AddressCidr,
};

/// An interface for sending and receiving network packets.
pub struct Interface {
    /// Device for sending and receiving raw Ethernet frames.
    pub dev: Box<Device>,
    /// Cache for IPv4/Ethernet address translations.
    pub arp_cache: ArpCache,
    /// Ethernet address for the interface.
    pub ethernet_addr: EthernetAddress,
    /// IPv4 address for the interface.
    pub ipv4_addr: Ipv4AddressCidr,
    /// Default gateway for IPv4 packets not on the interface subnet. This
    /// should be on the same subnet as ipv4_addr!
    pub default_gateway: Ipv4Address,
}
