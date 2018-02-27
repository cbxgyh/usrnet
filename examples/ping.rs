extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod env;

use usrnet::core::layers::{
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

lazy_static! {
    static ref IP_ADDR_PING: Ipv4Address = Ipv4Address::new([10, 0, 0, 1]);
}

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let mut service = env::default_service();

    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    // Send a ping request.
    let icmp_repr = Icmpv4Repr::EchoRequest { id: 42, seq: 1 };

    let ip_repr = Ipv4Repr {
        src_addr: env::default_ipv4_addr(),
        dst_addr: *IP_ADDR_PING,
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
            icmp_repr.serialize(&mut icmp_packet);
        })
        .unwrap();

    println!(
        "Sent ICMP ping to {}. Use tshark or tcpdump to observe.",
        *IP_ADDR_PING
    );

    // Loop until ping reply arrives.
    loop {
        while let Ok(ip_buffer) = socket_set.socket(raw_handle).as_raw_socket().recv() {
            let ip_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
            if ip_packet.protocol() != ipv4_protocols::ICMP || ip_packet.src_addr() != *IP_ADDR_PING
                || ip_packet.dst_addr() != env::default_ipv4_addr()
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
                    println!("Got ping response from {}!", *IP_ADDR_PING);
                    std::process::exit(0);
                }
                _ => continue,
            };
        }

        env::tick(&mut service, &mut socket_set);
    }
}
