use std::net::{
    IpAddr as StdIpAddr,
    Ipv4Addr as StdIpv4Addr,
};

use get_if_addrs;

use core::arp_cache::ArpCache;
use core::dev::Device;
use core::repr::{
    EthernetAddress,
    Ipv4Address,
    Ipv4AddressCidr,
};
use core::service::{
    socket,
    Interface,
};
use core::socket::{
    SocketEnv,
    SocketSet,
};
use core::time::SystemEnv;

/// Default capacity of a socket set.
pub static SOCKET_SET_HANDLES: usize = 64;

lazy_static! {
    /// Default interface IPv4 address.
    pub static ref DEFAULT_IPV4_ADDR: Ipv4Address = {
        Ipv4Address::new([10, 0, 0, 102])
    };

    /// An IPv4 address not assigned to any hosts on the network.
    pub static ref NO_HOST_IPV4_ADDR: Ipv4Address = {
        Ipv4Address::new([10, 0, 0, 64])
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
pub fn default_dev() -> Box<Device> {
    use linux::dev::Tap;
    Box::new(Tap::new("tap0"))
}

#[cfg(not(target_os = "linux"))]
pub fn default_dev() -> Box<Device> {
    panic!("Sorry, examples are only supported on Linux.");
}

/// Get's the IPv4 address for an interface. See tap.sh for more info.
pub fn ifr_addr(ifr_name: &str) -> StdIpv4Addr {
    for interface in get_if_addrs::get_if_addrs().unwrap() {
        if interface.name == ifr_name {
            if let StdIpAddr::V4(ipv4_addr) = interface.ip() {
                return ipv4_addr;
            }
        }
    }

    panic!("IPv4 address for '{}' not found!", ifr_name);
}

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

/// Creates a socket environment.
pub fn socket_env(interface: &mut Interface) -> SocketEnv<SystemEnv> {
    SocketEnv::new(interface, SystemEnv::new())
}

/// Creates a socket set.
pub fn socket_set() -> SocketSet {
    SocketSet::new(SOCKET_SET_HANDLES)
}

/// Sends and receives packets from/to sockets and the interface.
pub fn tick<'a>(interface: &mut Interface, socket_set: &mut SocketSet) {
    socket::recv(interface, socket_set);
    socket::send(interface, socket_set);
}
