extern crate usrnet;

use usrnet::core::dev::{
    Device,
    Standard,
};
use usrnet::core::link::{
    EthernetLink,
    Ipv4Link,
    Link,
};
use usrnet::core::repr::{
    Arp,
    ArpOp,
    EthernetFrame,
    EthernetType,
    Ipv4,
    Mac,
};
use usrnet::linux::link::Tap;

fn main() {
    let link = Box::new(Tap::new("tap0"));

    println!("Link MTU:  {}", link.get_max_transmission_unit().unwrap());
    println!("Link IPv4: {}", link.get_ipv4_addr().unwrap());
    println!("Link MAC:  {}", link.get_ethernet_addr().unwrap());

    let host_eth_addr = link.get_ethernet_addr().unwrap();

    let arp = Arp::EthernetIpv4 {
        op: ArpOp::Request,
        source_hw_addr: host_eth_addr,
        source_proto_addr: link.get_ipv4_addr().unwrap(),
        target_hw_addr: Mac::BROADCAST,
        target_proto_addr: Ipv4::new([172, 28, 128, 4]),
    };

    let mut dev = Standard::new(link).unwrap();

    let buffer_len = EthernetFrame::<&[u8]>::buffer_len(arp.buffer_len());
    let buffer = dev.send(buffer_len).unwrap();

    let mut eth_frame = EthernetFrame::new(buffer).unwrap();
    eth_frame.set_dst_addr(Mac::BROADCAST);
    eth_frame.set_src_addr(host_eth_addr);
    eth_frame.set_payload_type(EthernetType::Arp);

    arp.serialize(eth_frame.payload_mut()).unwrap();

    println!("ARP request sent. Use tshark or tcpdump to observe.");
}
