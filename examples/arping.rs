extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod env;

use usrnet::core::layers::{
    eth_types,
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

lazy_static! {
    static ref IP_ADDR_ARP: Ipv4Address = Ipv4Address::new([10, 0, 0, 1]);

    static ref TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1000);
}

/// Sends an ARP request for an IPv4 address.
fn main() {
    env_logger::init();

    let mut service = env::default_service();

    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ethernet));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    // Send an ARP request.
    let arp = Arp::EthernetIpv4 {
        op: ArpOp::Request,
        source_hw_addr: env::default_eth_addr(),
        source_proto_addr: env::default_ipv4_addr(),
        target_hw_addr: EthernetAddress::BROADCAST,
        target_proto_addr: *IP_ADDR_ARP,
    };

    let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(arp.buffer_len());

    socket_set
        .socket(raw_handle)
        .as_raw_socket()
        .send(eth_frame_len)
        .map(|eth_buffer| {
            let mut eth_frame = EthernetFrame::try_new(eth_buffer).unwrap();
            eth_frame.set_src_addr(env::default_eth_addr());
            eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
            eth_frame.set_payload_type(eth_types::ARP);
            arp.serialize(eth_frame.payload_mut()).unwrap();
        })
        .unwrap();

    println!("ARP request sent. Use tshark or tcpdump to observe.");

    let since = std::time::Instant::now();

    // Read frames until (1) ARP reply is received or (2) timeout.
    while std::time::Instant::now().duration_since(since) < *TIMEOUT {
        while let Ok(eth_buffer) = socket_set.socket(raw_handle).as_raw_socket().recv() {
            let eth_frame = EthernetFrame::try_new(eth_buffer).unwrap();
            if eth_frame.payload_type() != eth_types::ARP {
                continue;
            }

            match Arp::deserialize(eth_frame.payload()) {
                Ok(Arp::EthernetIpv4 {
                    op,
                    source_hw_addr,
                    source_proto_addr,
                    ..
                }) => {
                    if op == ArpOp::Reply && source_proto_addr == *IP_ADDR_ARP {
                        println!("{} has MAC {}!", source_proto_addr, source_hw_addr);
                        std::process::exit(0);
                    }
                }
                _ => continue,
            };
        }

        env::tick(&mut service, &mut socket_set);
    }

    eprintln!("Timeout!");
}
