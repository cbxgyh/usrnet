extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::core::layers::{
    Ipv4Packet,
    Ipv4Repr,
    UdpPacket,
    UdpRepr,
    ipv4_protocols,
};
use usrnet::core::socket::{
    RawType,
    SocketSet,
    TaggedSocket,
};

/// Echo's all incoming UDP packets back to the sender.
fn main() {
    env_logger::init();

    let mut service = env::default_service();

    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    let mut buffer = [0; 4096];

    println!("Running UDP echo server; You can use udp_echo_client.py to generate UDP packets.");

    // Loop to read and echo UDP packets.
    loop {
        let (mut ip_repr, mut udp_repr, payload_len) =
            recv_udp_packet(&mut socket_set, raw_handle, &mut service, &mut buffer[..]);

        // Echo in opposite direction.
        std::mem::swap(&mut ip_repr.src_addr, &mut ip_repr.dst_addr);
        std::mem::swap(&mut udp_repr.src_port, &mut udp_repr.dst_port);

        println!(
            "Echo {:?} via {}:{} -> {}:{}!",
            &buffer[..payload_len],
            ip_repr.src_addr,
            udp_repr.src_port,
            ip_repr.dst_addr,
            udp_repr.dst_port
        );

        let ip_packet_len = Ipv4Packet::<&[u8]>::buffer_len(udp_repr.length as usize);

        // NOTE: Sending of IP packets from socket is handled via env::tick in recv_udp_packet.
        socket_set
            .socket(raw_handle)
            .as_raw_socket()
            .send(ip_packet_len)
            .map(|ip_buffer| {
                let mut ip_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
                ip_repr.serialize(&mut ip_packet);

                let mut udp_packet = UdpPacket::try_new(ip_packet.payload_mut()).unwrap();
                udp_repr.serialize(&mut udp_packet, &buffer[..payload_len], ip_repr);
            })
            .unwrap();
    }
}

fn recv_udp_packet(
    socket_set: &mut SocketSet,
    raw_handle: usize,
    service: &mut env::TService,
    buffer: &mut [u8],
) -> (Ipv4Repr, UdpRepr, usize) {
    loop {
        if let Ok(ip_buffer) = socket_set.socket(raw_handle).as_raw_socket().recv() {
            let ip_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
            let ip_repr = Ipv4Repr::deserialize(&ip_packet).unwrap();
            if ip_packet.protocol() != ipv4_protocols::UDP
                || ip_packet.dst_addr() != env::default_ipv4_addr()
            {
                continue;
            }

            let udp_packet = UdpPacket::try_new(ip_packet.payload()).unwrap();
            if let Err(_) = udp_packet.check_encoding(ip_repr) {
                continue;
            }

            match UdpRepr::deserialize(&udp_packet) {
                Ok(udp_repr) => {
                    let udp_payload_len = udp_packet.payload().len();
                    (&mut buffer[..udp_payload_len]).copy_from_slice(udp_packet.payload());
                    return (ip_repr, udp_repr, udp_payload_len);
                }
                Err(_) => continue,
            }
        }

        env::tick(service, socket_set);
    }
}
