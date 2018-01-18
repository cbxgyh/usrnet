extern crate clap;
extern crate usrnet;

mod env;

use usrnet::core::dev::{
    Device,
    Error as DevError,
};
use usrnet::core::layers::{
    Arp,
    ArpOp,
    EthernetFrame,
    EthernetType,
    Ipv4,
    Mac,
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
                .default_value("10.0.0.102")
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

    let ip = matches.value_of("ip").unwrap().parse::<Ipv4>().unwrap();
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
        if let Some(mac) = recv_arp(&mut dev, ip.clone()) {
            println!("{} has MAC {}!", ip, mac);
            return 0;
        } else if now.duration_since(since) > timeout {
            eprintln!("Timeout!");
            return 1;
        }
    };

    std::process::exit(recv_arp_loop());
}

fn send_arp<'a, T: Device<'a>>(dev: &'a mut T, ip: Ipv4) {
    let arp = Arp::EthernetIpv4 {
        op: ArpOp::Request,
        source_hw_addr: dev.get_ethernet_addr(),
        source_proto_addr: dev.get_ipv4_addr(),
        target_hw_addr: Mac::BROADCAST,
        target_proto_addr: ip,
    };

    let dev_eth_addr = dev.get_ethernet_addr();
    let buffer_len = EthernetFrame::<&[u8]>::buffer_len(arp.buffer_len());
    let mut buffer = dev.send(buffer_len).unwrap();

    let mut eth_frame = EthernetFrame::new(buffer.as_mut()).unwrap();
    eth_frame.set_dst_addr(Mac::BROADCAST);
    eth_frame.set_src_addr(dev_eth_addr);
    eth_frame.set_payload_type(EthernetType::Arp);
    arp.serialize(eth_frame.payload_mut()).unwrap();
}

fn recv_arp<'a, T: Device<'a>>(dev: &'a mut T, ip: Ipv4) -> Option<Mac> {
    let eth_addr = dev.get_ethernet_addr();
    let ip_addr = dev.get_ipv4_addr();
    let maybe_buffer = dev.recv();

    if maybe_buffer.is_err() {
        match maybe_buffer.err().unwrap() {
            DevError::Nothing => {}
            err => eprintln!("Error: {:?}", err),
        }
        return None;
    }

    let buffer = maybe_buffer.unwrap();
    let eth_frame = EthernetFrame::new(buffer.as_ref()).unwrap();
    let payload_type = eth_frame.get_payload_type();

    if payload_type.is_err() || payload_type.unwrap() != EthernetType::Arp {
        return None; // Ignore non ARP packets...
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
                None // Ignore irrelevant ARP messages...
            } else {
                Some(source_hw_addr)
            }
        }
        _ => None,
    }
}
