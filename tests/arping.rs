#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate usrnet;

mod context;

use std::time::Duration;

use usrnet::core::repr::{
    EthernetAddress,
    Ipv4Address,
};
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};
use usrnet::examples::*;

fn arping_addr(
    context: &mut context::Context,
    addr: Ipv4Address,
) -> Option<(Duration, EthernetAddress)> {
    let raw_socket = context.socket_env.raw_socket(RawType::Ethernet);
    let raw_handle = context
        .socket_set
        .add_socket(TaggedSocket::Raw(raw_socket))
        .unwrap();

    arping(
        &mut context.interface,
        &mut context.socket_set,
        raw_handle,
        addr,
        *context::ONE_SEC,
    )
}

#[test]
fn arping_default_gateway() {
    context::run(|context| {
        let (time, _) = arping_addr(context, *env::DEFAULT_IPV4_GATEWAY).unwrap();
        assert!(time < *context::ONE_SEC);
    });
}

#[test]
fn arping_unknown_ip() {
    context::run(|context| {
        assert!(arping_addr(context, Ipv4Address::new([10, 0, 0, 128])).is_none());
    });
}
