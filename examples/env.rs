use std::thread;
use std::time::Duration;
use std::vec::Vec;

use usrnet::core::arp_cache::ArpCache;
use usrnet::core::repr::{
    EthernetAddress,
    Ipv4Address,
    Ipv4AddressCidr,
};
use usrnet::core::services::{
    socket,
    Interface,
};
use usrnet::core::socket::{
    AddrLease,
    RawSocket,
    RawType,
    SocketAddr,
    SocketSet,
    UdpSocket,
};
use usrnet::core::storage::{
    Ring,
    Slice,
};
use usrnet::core::time::SystemEnv;

lazy_static! {
    pub static ref DEFAULT_IPV4_ADDR: Ipv4Address = {
        Ipv4Address::new([10, 0, 0, 102])
    };

    pub static ref DEFAULT_IPV4_ADDR_CIDR: Ipv4AddressCidr = {
        Ipv4AddressCidr::new(*DEFAULT_IPV4_ADDR, 24)
    };

    pub static ref DEFAULT_IPV4_GATEWAY: Ipv4Address = {
        Ipv4Address::new([10, 0, 0, 101])
    };

    pub static ref DEFAULT_ETH_ADDR: EthernetAddress = {
        // Use a local MAC address!
        EthernetAddress::new([0x06, 0x11, 0x22, 0x33, 0x44, 0x55])
    };
}

#[cfg(target_os = "linux")]
mod platform {
    use usrnet::core::dev::Device;
    use usrnet::linux::dev::Tap;

    #[allow(dead_code)]
    pub fn default_dev() -> Box<Device> {
        Box::new(Tap::new("tap0"))
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use usrnet::core::dev::Device;

    #[allow(dead_code)]
    pub fn default_dev() -> Box<Device> {
        panic!("Sorry, examples are only supported on Linux.");
    }
}

pub use self::platform::default_dev;

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn socket_set<'a, 'b: 'a>() -> SocketSet<'a, 'b> {
    let mut sockets = vec![];
    for _ in 0 .. 16 {
        sockets.push(None);
    }
    SocketSet::new(Slice::from(sockets))
}

#[allow(dead_code)]
pub fn socket_buffer<'a, F, T>(len: usize, mut f: F) -> Ring<'a, T>
where
    F: FnMut() -> T,
{
    let items: Vec<_> = (0 .. len).map(|_| f()).collect();
    Ring::from(items)
}

#[allow(dead_code)]
pub fn raw_socket<'a>(raw_type: RawType) -> RawSocket<'a> {
    let buffer = || socket_buffer(32, || Slice::from(vec![0; 1500]));

    RawSocket::new(raw_type, buffer(), buffer())
}

#[allow(dead_code)]
pub fn udp_socket<'a>(binding: AddrLease<'a>) -> UdpSocket<'a> {
    let addr = SocketAddr {
        addr: Ipv4Address::new([0, 0, 0, 0]),
        port: 0,
    };

    let buffer = || socket_buffer(32, || (Slice::from(vec![0; 1500]), addr.clone()));

    UdpSocket::new(binding, buffer(), buffer())
}

#[allow(dead_code)]
pub fn tick<'a>(interface: &mut Interface, socket_set: &mut SocketSet) {
    thread::sleep(Duration::new(0, 1_000));
    socket::recv(interface, socket_set);
    socket::send(interface, socket_set);
}
