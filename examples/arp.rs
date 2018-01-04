extern crate usrnet;

use std::borrow::BorrowMut;

use usrnet::core::addr::{Ipv4, Mac, ETH_BROADCAST};
use usrnet::core::dev::{Device, Standard};
use usrnet::core::link::Link;
use usrnet::core::net::arp::{Arp, Op};
use usrnet::linux::dev::Tap;

const HOST_IP: Ipv4 = [0, 0, 0, 0];

const HOST_MAC: Mac = [0, 0, 0, 0, 0, 0];

const TARGET_IP: Ipv4 = [0, 0, 0, 0];

fn main() {
    let arp = Arp::EthernetIpv4 {
        op: Op::Request,
        source_hw_addr: HOST_MAC,
        source_proto_addr: HOST_IP,
        target_hw_addr: ETH_BROADCAST,
        target_proto_addr: TARGET_IP,
    };

    let link = Box::new(Tap::new("tap0"));

    println!("Link MTU: {}", link.max_transmission_unit().unwrap());

    let mut dev = Standard::new(link).unwrap();
    let mut buf = dev.send(arp.buffer_len()).unwrap();
    arp.serialize(buf.borrow_mut()).unwrap();

    println!("ARP request sent. Use tshark or tcpdump to observe.");
}
