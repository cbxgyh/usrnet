extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::core::layers::{
    ethernet_types,
    Arp,
    ArpOp,
    EthernetAddress,
    EthernetFrame,
    Ipv4Address,
};
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};

const IP_ADDR_ARP: [u8; 4] = [10, 0, 0, 1];

const TIMEOUT_MS: u64 = 1000;

/// Sends an ARP request for an IPv4 address.
fn main() {
    env_logger::init();

    let arp_ip = Ipv4Address::new(IP_ADDR_ARP);
    let timeout = std::time::Duration::from_millis(TIMEOUT_MS);

    let mut service = env::default_service();

    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ethernet));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    let arp = Arp::EthernetIpv4 {
        op: ArpOp::Request,
        source_hw_addr: env::default_eth_addr(),
        source_proto_addr: env::default_ipv4_addr(),
        target_hw_addr: EthernetAddress::BROADCAST,
        target_proto_addr: arp_ip,
    };

    socket_set
        .socket(raw_handle)
        .and_then(|socket| socket.as_raw_socket())
        .map(|socket| {
            let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(arp.buffer_len());
            let eth_buffer = socket.send(eth_frame_len).unwrap();
            let mut eth_frame = EthernetFrame::try_from(eth_buffer).unwrap();
            eth_frame.set_src_addr(env::default_eth_addr());
            eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
            eth_frame.set_payload_type(ethernet_types::ARP);
            arp.serialize(eth_frame.payload_mut()).unwrap();
        })
        .unwrap();

    service.send(&mut socket_set);
    println!("ARP request sent. Use tshark or tcpdump to observe.");

    let since = std::time::Instant::now();

    // Read frames until (1) ARP reply is received or (2) timeout.
    loop {
        let now = std::time::Instant::now();

        if now.duration_since(since) > timeout {
            eprintln!("Timeout!");
            return;
        }

        while let Some(eth_buffer) = socket_set
            .socket(raw_handle)
            .and_then(|socket| socket.as_raw_socket())
            .and_then(|socket| socket.recv().ok())
        {
            let eth_frame = EthernetFrame::try_from(eth_buffer).unwrap();

            if eth_frame.payload_type() != ethernet_types::ARP {
                continue;
            }

            match Arp::deserialize(eth_frame.payload()) {
                Ok(Arp::EthernetIpv4 {
                    op,
                    source_hw_addr,
                    source_proto_addr,
                    ..
                }) => {
                    if op == ArpOp::Reply && source_proto_addr == arp_ip {
                        println!("{} has MAC {}!", arp_ip, source_hw_addr);
                        return;
                    }
                }
                _ => continue,
            };
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        service.recv(&mut socket_set);
    }
}
