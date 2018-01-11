extern crate clap;
extern crate usrnet;

mod cli;

use cli::App;
use usrnet::core::dev::{
    Device,
    Standard,
};
use usrnet::core::link::Link;
use usrnet::core::repr::{
    Arp,
    ArpOp,
    EthernetFrame,
    EthernetType,
    Ipv4,
    Mac,
};
use usrnet::linux::link::Tap;

/// Sends an ARP request for an IPv4 address.
fn main() {
    let matches = clap::App::new("arping")
        .about("Sends an ARP request for an IPv4 address through a Linux TAP interface")
        .with_defaults()
        .get_matches();
    let interface = matches.value_of("tap").unwrap();
    let tap = Tap::new(interface);
    let mtu = tap.get_max_transmission_unit().unwrap();

    let mut dev = Standard::new(
        tap,
        matches
            .value_of("dev-ipv4")
            .unwrap()
            .parse::<Ipv4>()
            .unwrap(),
        matches.value_of("dev-mac").unwrap().parse::<Mac>().unwrap(),
    ).unwrap();

    println!("Link MTU:    {}", mtu);
    println!("Device IPv4: {}", dev.get_ipv4_addr());
    println!("Device MAC:  {}", dev.get_ethernet_addr());

    let arp = Arp::EthernetIpv4 {
        op: ArpOp::Request,
        source_hw_addr: dev.get_ethernet_addr(),
        source_proto_addr: dev.get_ipv4_addr(),
        target_hw_addr: Mac::BROADCAST,
        target_proto_addr: Ipv4::new([10, 0, 0, 102]),
    };

    let dev_eth_addr = dev.get_ethernet_addr();

    let buffer_len = EthernetFrame::<&[u8]>::buffer_len(arp.buffer_len());
    let buffer = dev.send(buffer_len).unwrap();

    let mut eth_frame = EthernetFrame::new(buffer).unwrap();
    eth_frame.set_dst_addr(Mac::BROADCAST);
    eth_frame.set_src_addr(dev_eth_addr);
    eth_frame.set_payload_type(EthernetType::Arp);

    arp.serialize(eth_frame.payload_mut()).unwrap();

    drop(eth_frame); // Send ARP

    println!("ARP request sent. Use tshark or tcpdump to observe.");

    std::thread::sleep(std::time::Duration::from_secs(1));
}
