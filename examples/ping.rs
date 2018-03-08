extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod env;

use std::process;
use std::str::FromStr;

use clap::{
    App,
    Arg,
};

use usrnet::core::repr::{
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    Ipv4Packet,
    Ipv4Protocol,
    Ipv4Repr,
    ipv4_protocols,
};
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let matches = App::new("ping")
        .arg(
            Arg::with_name("ADDRESS")
                .value_name("ADDRESS")
                .takes_value(true),
        )
        .get_matches();

    let ping_addr = matches
        .value_of("ADDRESS")
        .map(|addr| Ipv4Address::from_str(addr).unwrap())
        .unwrap_or(*env::DEFAULT_IPV4_GATEWAY);

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    // Send a ping request.
    let icmp_repr = Icmpv4Repr::EchoRequest { id: 42, seq: 1 };

    let ip_repr = Ipv4Repr {
        src_addr: *env::DEFAULT_IPV4_ADDR,
        dst_addr: ping_addr,
        protocol: Ipv4Protocol::ICMP,
        payload_len: icmp_repr.buffer_len() as u16,
    };

    socket_set
        .socket(raw_handle)
        .as_raw_socket()
        .send(ip_repr.buffer_len())
        .map(|ip_buffer| {
            let mut ip_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
            ip_repr.serialize(&mut ip_packet);

            let mut icmp_packet = Icmpv4Packet::try_new(ip_packet.payload_mut()).unwrap();
            icmp_repr.serialize(&mut icmp_packet).unwrap();
        })
        .unwrap();

    println!(
        "Sent ICMP ping to {}. Use tshark or tcpdump to observe.",
        ping_addr
    );

    // Loop until ping reply arrives.
    loop {
        while let Ok(ip_buffer) = socket_set.socket(raw_handle).as_raw_socket().recv() {
            let ip_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
            if ip_packet.protocol() != ipv4_protocols::ICMP || ip_packet.src_addr() != ping_addr
                || ip_packet.dst_addr() != *env::DEFAULT_IPV4_ADDR
            {
                continue;
            }

            let icmp_packet = Icmpv4Packet::try_new(ip_packet.payload()).unwrap();
            if let Err(_) = icmp_packet.check_encoding() {
                continue;
            }

            let icmp_repr = Icmpv4Repr::deserialize(&icmp_packet);
            match icmp_repr {
                Ok(Icmpv4Repr::EchoReply { .. }) => {
                    println!("Got ping response from {}!", ping_addr);
                    process::exit(0);
                }
                _ => continue,
            };
        }

        env::tick(&mut interface, &mut socket_set);
    }
}
