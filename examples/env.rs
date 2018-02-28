use std;

use usrnet::core::arp_cache::ArpCache;
use usrnet::core::dev::Device;
use usrnet::core::layers::{
    EthernetAddress,
    Ipv4Address,
};
use usrnet::core::service::Service;
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
use usrnet::linux::dev::Tap;

pub type TDev = Tap;

pub type TService = Service<TDev>;

pub fn default_ipv4_addr() -> Ipv4Address {
    Ipv4Address::new([10, 0, 0, 103])
}

pub fn default_eth_addr() -> EthernetAddress {
    EthernetAddress::new([0, 1, 2, 3, 4, 5])
}

#[allow(dead_code)]
pub fn default_dev() -> TDev {
    let tap = Tap::new("tap0", default_ipv4_addr(), default_eth_addr());

    println!(
        "Device: (MTU = {}, IPv4 = {}, MAC = {})",
        tap.max_transmission_unit(),
        tap.ipv4_addr(),
        tap.ethernet_addr()
    );

    tap
}

#[allow(dead_code)]
pub fn default_service() -> TService {
    let dev = default_dev();
    let arp_cache = ArpCache::new(60, SystemEnv::new());
    Service::new(dev, arp_cache)
}

#[allow(dead_code)]
pub fn socket_set<'a, 'b: 'a>() -> SocketSet<'a, 'b> {
    let mut sockets = vec![];
    for _ in 0..16 {
        sockets.push(None);
    }
    SocketSet::new(Slice::from(sockets))
}

#[allow(dead_code)]
pub fn socket_buffer<'a, F, T>(len: usize, mut f: F) -> Ring<'a, T>
where
    F: FnMut() -> T,
{
    let items: std::vec::Vec<_> = (0..len).map(|_| f()).collect();
    Ring::from(items)
}

#[allow(dead_code)]
pub fn raw_socket<'a>(raw_type: RawType) -> RawSocket<'a> {
    let buffer = || socket_buffer(32, || Slice::from(vec![0; 1500]));

    RawSocket::new(buffer(), buffer(), raw_type)
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
pub fn tick<'a>(service: &mut TService, socket_set: &mut SocketSet) {
    std::thread::sleep(std::time::Duration::from_millis(10));
    service.recv(socket_set);
    service.send(socket_set);
}
