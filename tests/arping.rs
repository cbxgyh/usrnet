#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod context;

use std::time::Duration;

use usrnet::core::repr::{
    EthernetAddress,
    Ipv4Address,
};
use usrnet::core::service::Interface;
use usrnet::core::socket::{
    RawType,
    SocketSet,
    TaggedSocket,
};
use usrnet::examples::*;

lazy_static! {
    static ref TIMEOUT: Duration = Duration::from_secs(1);
}

fn arping_addr(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    addr: Ipv4Address,
) -> Option<(Duration, EthernetAddress)> {
    let raw_socket = TaggedSocket::Raw(env::raw_socket(interface, RawType::Ethernet));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    arping(interface, socket_set, raw_handle, addr, *TIMEOUT)
}

#[test]
fn arping_default_gateway() {
    context::run(|interface, socket_set| {
        let (time, _) = arping_addr(interface, socket_set, *env::DEFAULT_IPV4_GATEWAY).unwrap();
        assert!(time < *TIMEOUT);
    });
}

#[test]
fn arping_unknown_ip() {
    context::run(|interface, socket_set| {
        assert!(arping_addr(interface, socket_set, Ipv4Address::new([10, 0, 0, 128])).is_none());
    });
}
