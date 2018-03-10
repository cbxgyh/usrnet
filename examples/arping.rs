extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod env;

use std::process;
use std::str::FromStr;
use std::time::{
    Duration,
    Instant,
};

use clap::{
    App,
    Arg,
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
    static ref TIMEOUT: Duration = Duration::from_millis(1000);
}

/// Sends an ARP request for an IPv4 address.
fn main() {
    env_logger::init();

    let matches = App::new("ping")
        .arg(
            Arg::with_name("ADDRESS")
                .value_name("ADDRESS")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let arping_addr = matches
        .value_of("ADDRESS")
        .map(|addr| Ipv4Address::from_str(addr).unwrap())
        .expect("Bad IP address!");

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ethernet));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    // Send an ARP request.
    let arp = Arp {
        op: ArpOp::Request,
        source_hw_addr: interface.ethernet_addr,
        source_proto_addr: *interface.ipv4_addr,
        target_hw_addr: EthernetAddress::BROADCAST,
        target_proto_addr: arping_addr,
    };

    let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(arp.buffer_len());

    socket_set
        .socket(raw_handle)
        .as_raw_socket()
        .send(eth_frame_len)
        .map(|eth_buffer| {
            let mut eth_frame = EthernetFrame::try_new(eth_buffer).unwrap();
            eth_frame.set_src_addr(interface.ethernet_addr);
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
                    if arp_repr.op == ArpOp::Reply && arp_repr.source_proto_addr == arping_addr {
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
    process::exit(1);
}
