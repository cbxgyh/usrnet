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
use core::layers::Ipv4Address;

/// An interface for sending and receiving network packets.
pub struct Interface {
    /// A device for sending and receiving raw Ethernet frames.
    pub dev: Box<Device>,
    /// A cache for IPv4/Ethernet address translations.
    pub arp_cache: ArpCache,
    /// A default gateway for IPv4 packets not on the device's local network.
    pub default_gateway: Ipv4Address,
}
