extern crate clap;
extern crate usrnet;

mod env;

use usrnet::core::dev::{
    Device,
    Error as DevError,
};
use usrnet::core::layers::{
    ethernet_types,
    Arp,
    ArpOp,
    EthernetAddress,
    EthernetFrame,
    Ipv4Address,
};

/// Sends an ARP request for an IPv4 address.
fn main() {
    let matches = clap::App::new("arping")
        .about("Sends an ARP request for an IPv4 address through a Linux TAP interface")
        .arg(
            clap::Arg::with_name("ip")
                .long("ip")
                .value_name("IP")
                .help("IP address to request a MAC address for")
                .default_value("10.0.0.1")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("timeout")
                .long("timeout")
                .value_name("TIMEOUT")
                .help("Timeout in MS when waiting for an ARP reply")
                .default_value("1000")
                .takes_value(true),
        )
        .get_matches();

    let ip = matches
        .value_of("ip")
        .unwrap()
        .parse::<Ipv4Address>()
        .unwrap();
    let timeout = std::time::Duration::from_millis(
        matches.value_of("timeout").unwrap().parse::<u64>().unwrap(),
    );
    let mut dev = env::default_dev();

    send_arp(&mut dev, ip);
    println!("ARP request sent. Use tshark or tcpdump to observe.");

    let since = std::time::Instant::now();

    // Read frames until (1) ARP reply is received or (2) timeout.
    let mut recv_arp_loop = || loop {
        std::thread::sleep(std::time::Duration::from_millis(1));
        let now = std::time::Instant::now();
        if let Some(eth_addr) = recv_arp(&mut dev, ip.clone()) {
            println!("{} has MAC {}!", ip, eth_addr);
            return 0;
        } else if now.duration_since(since) > timeout {
            eprintln!("Timeout!");
            return 1;
        }
    };

    std::process::exit(recv_arp_loop());
}

fn send_arp(dev: &mut env::Dev, ip: Ipv4Address) {
    let arp = Arp::EthernetIpv4 {
        op: ArpOp::Request,
        source_hw_addr: dev.get_ethernet_addr(),
        source_proto_addr: dev.get_ipv4_addr(),
        target_hw_addr: EthernetAddress::BROADCAST,
        target_proto_addr: ip,
    };

    let dev_eth_addr = dev.get_ethernet_addr();
    let buffer_len = EthernetFrame::<&[u8]>::buffer_len(arp.buffer_len());
    dev.send(buffer_len, |buffer| {
        let mut eth_frame = EthernetFrame::try_from(buffer).unwrap();
        eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
        eth_frame.set_src_addr(dev_eth_addr);
        eth_frame.set_payload_type(ethernet_types::ARP as u16);
        arp.serialize(eth_frame.payload_mut()).unwrap();
    }).unwrap();
}

fn recv_arp(dev: &mut env::Dev, ip: Ipv4Address) -> Option<EthernetAddress> {
    let eth_addr = dev.get_ethernet_addr();
    let ip_addr = dev.get_ipv4_addr();

    match dev.recv(|buffer| {
        let eth_frame = EthernetFrame::try_from(buffer).unwrap();

        if eth_frame.payload_type() != ethernet_types::ARP {
            return None;
        }

        match Arp::deserialize(eth_frame.payload()) {
            Ok(Arp::EthernetIpv4 {
                op,
                source_hw_addr,
                source_proto_addr,
                target_hw_addr,
                target_proto_addr,
            }) => {
                if op != ArpOp::Reply || source_proto_addr != ip || target_hw_addr != eth_addr
                    || target_proto_addr != ip_addr
                {
                    None
                } else {
                    Some(source_hw_addr)
                }
            }
            _ => None,
        }
    }) {
        Err(DevError::Nothing) => None,
        Err(err) => {
            eprintln!("Error: {:?}", err);
            None
        }
        Ok(hw_addr) => hw_addr,
    }
}
