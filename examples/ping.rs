extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::core::layers::{
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    Ipv4Packet,
    ipv4_flags,
    ipv4_types,
};
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};

const IP_ADDR_PING: [u8; 4] = [10, 0, 0, 1];

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let mut service = env::default_service();

    let ip_addr_ping = Ipv4Address::new(IP_ADDR_PING);

    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    // Send a ping request.
    socket_set
        .socket(raw_handle)
        .and_then(|socket| socket.as_raw_socket())
        .map(|socket| {
            let icmp_packet_len = Icmpv4Packet::<&[u8]>::buffer_len(0);
            let ip_packet_len = Ipv4Packet::<&[u8]>::buffer_len(icmp_packet_len);

            let ip_buffer = socket.send(ip_packet_len).unwrap();
            let mut ip_packet = Ipv4Packet::try_from(ip_buffer).unwrap();
            ip_packet.set_ip_version(4);
            ip_packet.set_header_len(5);
            ip_packet.set_dscp(0);
            ip_packet.set_ecn(0);
            ip_packet.set_packet_len(ip_packet_len as u16);
            ip_packet.set_identification(0);
            ip_packet.set_flags(ipv4_flags::DONT_FRAGMENT);
            ip_packet.set_fragment_offset(0);
            ip_packet.set_ttl(64);
            ip_packet.set_protocol(ipv4_types::ICMP as u8);
            ip_packet.set_header_checksum(0);
            ip_packet.set_src_addr(env::default_ipv4_addr());
            ip_packet.set_dst_addr(ip_addr_ping);

            let mut icmp_packet = Icmpv4Packet::try_from(ip_packet.payload_mut()).unwrap();
            icmp_packet.set_checksum(0);

            let icmp_repr = Icmpv4Repr::EchoRequest { id: 42, seq: 1 };
            icmp_repr.serialize(&mut icmp_packet);

            let checksum = icmp_packet.gen_checksum();
            icmp_packet.set_checksum(checksum);
        })
        .unwrap();

    println!(
        "Sent ICMP ping to {}. Use tshark or tcpdump to observe.",
        ip_addr_ping
    );

    // Loop until ping reply arrives.
    loop {
        while let Some(ip_buffer) = socket_set
            .socket(raw_handle)
            .and_then(|socket| socket.as_raw_socket())
            .and_then(|socket| socket.recv().ok())
        {
            let ip_packet = Ipv4Packet::try_from(ip_buffer).unwrap();

            if ip_packet.protocol() != ipv4_types::ICMP || ip_packet.src_addr() != ip_addr_ping
                || ip_packet.dst_addr() != env::default_ipv4_addr()
            {
                continue;
            }

            let icmp_packet = Icmpv4Packet::try_from(ip_packet.payload()).unwrap();
            if icmp_packet.is_encoding_ok().is_err() {
                continue;
            }

            let icmp_repr = Icmpv4Repr::deserialize(&icmp_packet);
            match icmp_repr {
                Ok(Icmpv4Repr::EchoReply { .. }) => {
                    println!("Got ping response from {}!", ip_addr_ping);
                    return;
                }
                _ => continue,
            };
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        service.send(&mut socket_set);
        service.recv(&mut socket_set);
    }
}
