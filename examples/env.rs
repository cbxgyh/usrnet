use std;

use usrnet::core::arp_cache::ArpCache;
use usrnet::core::dev::Device;
use usrnet::core::layers::{
    EthernetAddress,
    Ipv4Address,
};
use usrnet::core::service::Service;
use usrnet::core::socket::{
    RawSocket,
    RawType,
    SocketSet,
};
use usrnet::core::storage::{
    Ring,
    Slice,
};
use usrnet::core::time::SystemEnv;
use usrnet::linux::dev::Tap;

pub type TDev = Tap;

pub type TService = Service<TDev>;

static mut DEV_BUFFER: [u8; 10240] = [0; 10240];

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
pub fn mut_buffer(buffer_len: usize) -> &'static mut [u8] {
    unsafe {
        let buffer = &mut DEV_BUFFER[..buffer_len];
        for i in 0..buffer_len {
            buffer[i] = 0
        }
        buffer
    }
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
pub fn raw_socket<'a>(raw_type: RawType) -> RawSocket<'a> {
    let ring = || {
        let mut buffers = std::vec::Vec::new();
        for _ in 0..32 {
            buffers.push(Slice::from(vec![0; 1500]));
        }
        Ring::from(buffers)
    };

    RawSocket::new(ring(), ring(), raw_type)
}

#[allow(dead_code)]
pub fn tick<'a>(service: &mut TService, socket_set: &mut SocketSet) {
    std::thread::sleep(std::time::Duration::from_millis(10));
    service.recv(socket_set);
    service.send(socket_set);
}
