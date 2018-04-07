use std::vec::Vec;

use core::arp_cache::ArpCache;
use core::repr::{
    EthernetAddress,
    EthernetFrame,
    Ipv4Address,
    Ipv4AddressCidr,
    Ipv4Packet,
    UdpPacket,
};
use core::service::{
    socket,
    Interface,
};
use core::socket::{
    AddrLease,
    RawSocket,
    RawType,
    SocketAddr,
    SocketSet,
    UdpSocket,
};
use core::storage::{
    Ring,
    Slice,
};
use core::time::SystemEnv;

/// Default number of handles a socket set has capacity for.
pub static SOCKET_SET_HANDLES: usize = 16;

/// Default number of packets a raw socket can buffer.
pub static RAW_SOCKET_BUFFER_LEN: usize = 128;

/// Default number of packets a UDP socket can buffer.
pub static UDP_SOCKET_BUFFER_LEN: usize = 128;

lazy_static! {
    /// Default interface IPv4 address.
    pub static ref DEFAULT_IPV4_ADDR: Ipv4Address = {
        Ipv4Address::new([10, 0, 0, 102])
    };

    /// Default interface IPv4 address with a subnet mask.
    pub static ref DEFAULT_IPV4_ADDR_CIDR: Ipv4AddressCidr = {
        Ipv4AddressCidr::new(*DEFAULT_IPV4_ADDR, 24)
    };

    /// Default interface IPv4 gateway.
    pub static ref DEFAULT_IPV4_GATEWAY: Ipv4Address = {
        Ipv4Address::new([10, 0, 0, 101])
    };

    /// Default interface MAC address.
    pub static ref DEFAULT_ETH_ADDR: EthernetAddress = {
        EthernetAddress::new([0x06, 0x11, 0x22, 0x33, 0x44, 0x55])
    };
}

#[cfg(target_os = "linux")]
mod platform {
    use core::dev::Device;
    use linux::dev::Tap;

    pub fn default_dev() -> Box<Device> {
        Box::new(Tap::new("tap0"))
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use core::dev::Device;

    pub fn default_dev() -> Box<Device> {
        panic!("Sorry, examples are only supported on Linux.");
    }
}

pub use self::platform::default_dev;

/// Creates a network interface.
pub fn default_interface() -> Interface {
    let interface = Interface {
        dev: default_dev(),
        arp_cache: ArpCache::new(60, SystemEnv::new()),
        ethernet_addr: *DEFAULT_ETH_ADDR,
        ipv4_addr: *DEFAULT_IPV4_ADDR_CIDR,
        default_gateway: *DEFAULT_IPV4_GATEWAY,
    };

    println!(
        "Interface: (MTU = {}, MAC = {}, IPv4 = {}, Gateway: {})",
        interface.dev.max_transmission_unit(),
        interface.ethernet_addr,
        interface.ipv4_addr,
        interface.default_gateway,
    );

    interface
}

/// Creates a ring of len elements using factory function f.
pub fn ring<'a, F, T>(len: usize, mut f: F) -> Ring<'a, T>
where
    F: FnMut() -> T,
{
    let items: Vec<_> = (0 .. len).map(|_| f()).collect();
    Ring::from(items)
}

/// Creates a socket set.
pub fn socket_set<'a, 'b: 'a>() -> SocketSet<'a, 'b> {
    let mut sockets = vec![];
    for _ in 0 .. SOCKET_SET_HANDLES {
        sockets.push(None);
    }
    SocketSet::new(Slice::from(sockets))
}

/// Creates a raw socket.
pub fn raw_socket<'a>(interface: &mut Interface, raw_type: RawType) -> RawSocket<'a> {
    let header_len = match raw_type {
        RawType::Ethernet => EthernetFrame::<&[u8]>::HEADER_LEN,
        RawType::Ipv4 => EthernetFrame::<&[u8]>::HEADER_LEN + Ipv4Packet::<&[u8]>::MIN_HEADER_LEN,
    };

    let payload_len = interface
        .dev
        .max_transmission_unit()
        .checked_sub(header_len)
        .unwrap();

    let buffer = || ring(RAW_SOCKET_BUFFER_LEN, || Slice::from(vec![0; payload_len]));

    RawSocket::new(raw_type, buffer(), buffer())
}

/// Creates a udp socket.
pub fn udp_socket<'a>(interface: &mut Interface, binding: AddrLease<'a>) -> UdpSocket<'a> {
    let addr = SocketAddr {
        addr: Ipv4Address::new([0, 0, 0, 0]),
        port: 0,
    };

    let udp_payload_len = interface
        .dev
        .max_transmission_unit()
        .checked_sub(UdpPacket::<&[u8]>::HEADER_LEN)
        .unwrap();

    let buffer = || {
        ring(UDP_SOCKET_BUFFER_LEN, || {
            (Slice::from(vec![0; udp_payload_len]), addr.clone())
        })
    };

    UdpSocket::new(binding, buffer(), buffer())
}

/// Sends and receives packets from/to sockets and the interface.
pub fn tick<'a>(interface: &mut Interface, socket_set: &mut SocketSet) {
    socket::recv(interface, socket_set);
    socket::send(interface, socket_set);
}
