extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod env;

use std::process;
use std::time::{
    Duration,
    Instant,
};

use usrnet::core::repr::{
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
    static ref IP_ADDR_ARP: Ipv4Address = *env::DEFAULT_IPV4_GATEWAY;

    static ref TIMEOUT: Duration = Duration::from_millis(1000);
}

/// Sends an ARP request for an IPv4 address.
fn main() {
    env_logger::init();

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ethernet));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    // Send an ARP request.
    let arp = Arp {
        op: ArpOp::Request,
        source_hw_addr: *env::DEFAULT_ETH_ADDR,
        source_proto_addr: *env::DEFAULT_IPV4_ADDR,
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
            eth_frame.set_src_addr(*env::DEFAULT_ETH_ADDR);
            eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
            eth_frame.set_payload_type(eth_types::ARP);
            arp.serialize(eth_frame.payload_mut()).unwrap();
        })
        .unwrap();

    println!("ARP request sent. Use tshark or tcpdump to observe.");

    let since = Instant::now();

    // Read frames until (1) ARP reply is received or (2) timeout.
    while Instant::now().duration_since(since) < *TIMEOUT {
        while let Ok(eth_buffer) = socket_set.socket(raw_handle).as_raw_socket().recv() {
            let eth_frame = EthernetFrame::try_new(eth_buffer).unwrap();
            if eth_frame.payload_type() != eth_types::ARP {
                continue;
            }

            match Arp::deserialize(eth_frame.payload()) {
                Ok(arp_repr) => {
                    if arp_repr.op == ArpOp::Reply && arp_repr.source_proto_addr == *IP_ADDR_ARP {
                        println!(
                            "{} has MAC {}!",
                            arp_repr.source_proto_addr, arp_repr.source_hw_addr
                        );
                        process::exit(0);
                    }
                }
                _ => continue,
            }
        }

        env::tick(&mut interface, &mut socket_set);
    }

    eprintln!("Timeout!");
}
