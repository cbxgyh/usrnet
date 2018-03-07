use std::thread;
use std::time::Duration;
use std::vec::Vec;

use usrnet::core::arp_cache::ArpCache;
use usrnet::core::layers::{
    EthernetAddress,
    Ipv4Address,
    Ipv4AddressCidr,
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
mod dev {
    use usrnet::core::dev::Device;
    use usrnet::linux::dev::Tap;

    pub type TDev = Tap;

    #[allow(dead_code)]
    pub fn default_dev() -> TDev {
        let tap = Tap::new(
            "tap0",
            *super::DEFAULT_IPV4_ADDR_CIDR,
            *super::DEFAULT_ETH_ADDR,
        );

        println!(
            "Device: (MTU = {}, IPv4 = {}, MAC = {})",
            tap.max_transmission_unit(),
            tap.ipv4_addr(),
            tap.ethernet_addr()
        );

        tap
    }
}

#[cfg(not(target_os = "linux"))]
mod dev {
    use usrnet::Result;
    use usrnet::core::dev::Device;
    use usrnet::core::layers::{
        EthernetAddress,
        Ipv4AddressCidr,
    };

    pub struct TDev {}

    impl Device for TDev {
        fn send(&mut self, _: &[u8]) -> Result<()> {
            unimplemented!()
        }

        fn recv(&mut self, _: &mut [u8]) -> Result<usize> {
            unimplemented!()
        }

        fn max_transmission_unit(&self) -> usize {
            unimplemented!()
        }

        fn ipv4_addr(&self) -> Ipv4AddressCidr {
            unimplemented!()
        }

        fn ethernet_addr(&self) -> EthernetAddress {
            unimplemented!()
        }
    }

    #[allow(dead_code)]
    pub fn default_dev() -> TDev {
        panic!("Sorry, examples are only supported on Linux.");
    }
}

pub use self::dev::{
    default_dev,
    TDev,
};

pub type TService = Service<TDev>;

#[allow(dead_code)]
pub fn default_service() -> TService {
    let dev = default_dev();
    let arp_cache = ArpCache::new(60, SystemEnv::new());
    Service::new(dev, arp_cache, *DEFAULT_IPV4_GATEWAY)
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
pub fn tick<'a>(service: &mut TService, socket_set: &mut SocketSet) {
    thread::sleep(Duration::new(0, 1_000));
    service.recv(socket_set);
    service.send(socket_set);
}
