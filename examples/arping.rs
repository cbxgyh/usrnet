extern crate clap;
extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::{
    Error,
    Result,
};
use usrnet::core::layers::{
    ethernet_types,
    Arp,
    ArpOp,
    EthernetAddress,
    Ipv4Address,
};
use usrnet::core::socket::{
    RawSocket,
    Socket,
};

/// Sends an ARP request for an IPv4 address.
fn main() {
    env_logger::init();

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

    let arp_ip = matches
        .value_of("ip")
        .unwrap()
        .parse::<Ipv4Address>()
        .unwrap();

    let timeout = std::time::Duration::from_millis(
        matches.value_of("timeout").unwrap().parse::<u64>().unwrap(),
    );

    let mut service = env::default_service();

    let mut sockets = [Socket::RawSocket(env::raw_socket())];

    let arp = Arp::EthernetIpv4 {
        op: ArpOp::Request,
        source_hw_addr: env::default_eth_addr(),
        source_proto_addr: env::default_ipv4_addr(),
        target_hw_addr: EthernetAddress::BROADCAST,
        target_proto_addr: arp_ip,
    };

    sockets[0]
        .try_as_raw_socket()
        .unwrap()
        .send(arp.buffer_len(), |mut eth_frame| {
            eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
            eth_frame.set_payload_type(ethernet_types::ARP);
            arp.serialize(eth_frame.payload_mut()).unwrap();
        })
        .unwrap();

    service.send(&mut sockets);
    println!("ARP request sent. Use tshark or tcpdump to observe.");

    let since = std::time::Instant::now();

    // Read frames until (1) ARP reply is received or (2) timeout.
    let mut recv_arp_loop = || loop {
        let now = std::time::Instant::now();

        if now.duration_since(since) > timeout {
            eprintln!("Timeout!");
            return 1;
        }

        match recv_arp(sockets[0].try_as_raw_socket().unwrap(), arp_ip) {
            Ok(eth_addr) => {
                println!("{} has MAC {}!", arp_ip, eth_addr);
                return 0;
            }
            Err(Error::Exhausted) => {
                std::thread::sleep(std::time::Duration::from_millis(1));
                service.recv(&mut sockets);
            }
            Err(_) => continue,
        }
    };

    std::process::exit(recv_arp_loop());
}

fn recv_arp<'a>(raw_socket: &mut RawSocket<'a>, arp_ip: Ipv4Address) -> Result<EthernetAddress> {
    match raw_socket.recv(|eth_frame| {
        if eth_frame.payload_type() != ethernet_types::ARP {
            return None;
        }

        match Arp::deserialize(eth_frame.payload()) {
            Ok(Arp::EthernetIpv4 {
                op,
                source_hw_addr,
                source_proto_addr,
                ..
            }) => {
                if op != ArpOp::Reply || source_proto_addr != arp_ip {
                    None
                } else {
                    Some(source_hw_addr)
                }
            }
            _ => None,
        }
    }) {
        Ok(Some(eth_addr)) => Ok(eth_addr),
        Ok(None) => Err(Error::NoOp),
        Err(err) => Err(err),
    }
}
