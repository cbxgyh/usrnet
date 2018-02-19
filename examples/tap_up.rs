extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::core::layers::{
    ethernet_types,
    EthernetFrame,
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    Ipv4Packet,
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

    let ip_addr = Ipv4Address::new(IP_ADDR_PING);

    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ethernet));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    // Loop until ARP resolves IP address...
    loop {
        service.recv(&mut socket_set);

        let buffer_len = Icmpv4Packet::<&[u8]>::MIN_BUFFER_LEN;

        match service.send_ipv4_packet(buffer_len, ip_addr, |ipv4_packet| {
            ipv4_packet.set_protocol(ipv4_types::ICMP);
            ipv4_packet.set_dst_addr(ip_addr);

            let mut icmp_packet = Icmpv4Packet::try_from(ipv4_packet.payload_mut()).unwrap();
            icmp_packet.set_checksum(0);

            let icmp_repr = Icmpv4Repr::EchoRequest { id: 42, seq: 1 };
            icmp_repr.serialize(&mut icmp_packet);

            let checksum = icmp_packet.gen_checksum();
            icmp_packet.set_checksum(checksum);
        }) {
            Ok(_) => break,
            _ => {}
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Sent a ping to {} via Tap!", ip_addr);

    // Loop until ping reply arrives.
    loop {
        while let Some(eth_buffer) = socket_set
            .socket(raw_handle)
            .and_then(|socket| socket.as_raw_socket())
            .and_then(|socket| socket.recv().ok())
        {
            let eth_frame = EthernetFrame::try_from(eth_buffer).unwrap();

            if eth_frame.payload_type() != ethernet_types::IPV4 {
                continue;
            }

            let ip_packet = Ipv4Packet::try_from(eth_frame.payload()).unwrap();
            if ip_packet.is_encoding_ok().is_err() || ip_packet.protocol() != ipv4_types::ICMP
                || ip_packet.src_addr() != ip_addr
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
                    println!("Got ping response from {}!", ip_addr);
                    return;
                }
                _ => continue,
            };
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
        service.recv(&mut socket_set);
    }
}
