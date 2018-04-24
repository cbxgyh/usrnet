use std::time::{
    Duration,
    Instant,
};

use core::repr::{
    ipv4_protocols,
    Icmpv4Message,
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    Ipv4Packet,
    Ipv4Protocol,
    Ipv4Repr,
};
use core::service::Interface;
use core::socket::SocketSet;
use examples::env;
use Error;

/// Sends an ICMP ping request to a host via a raw IP socket.
pub fn ping(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    raw_handle: usize,
    ping_addr: Ipv4Address,
    id: u16,
    seq: u16,
    payload: &[u8],
    timeout: Duration,
) -> Option<Duration> {
    let icmp_repr = Icmpv4Repr {
        message: Icmpv4Message::EchoRequest { id, seq },
        payload_len: payload.len(),
    };

    let ipv4_repr = Ipv4Repr {
        src_addr: *interface.ipv4_addr,
        dst_addr: ping_addr,
        protocol: Ipv4Protocol::ICMP,
        payload_len: icmp_repr.buffer_len() as u16,
    };

    // Socket may have a full send buffer!
    while let Err(_) = socket_set
        .socket(raw_handle)
        .as_raw_socket()
        .send(ipv4_repr.buffer_len())
        .map(|ip_buffer| {
            let mut ipv4_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
            ipv4_repr.serialize(&mut ipv4_packet);

            let mut icmp_packet = Icmpv4Packet::try_new(ipv4_packet.payload_mut()).unwrap();
            icmp_repr.serialize(&mut icmp_packet).unwrap();
            icmp_packet.payload_mut().copy_from_slice(payload);
            icmp_packet.fill_checksum();
        }) {
        env::tick(interface, socket_set);
    }

    let send_at = Instant::now();

    loop {
        let waiting = Instant::now().duration_since(send_at);

        if waiting >= timeout {
            return None;
        } else if let Ok(_) = socket_set
            .socket(raw_handle)
            .as_raw_socket()
            .recv()
            .and_then(|ip_buffer| {
                let ipv4_packet = Ipv4Packet::try_new(ip_buffer)?;
                if ipv4_packet.protocol() != ipv4_protocols::ICMP
                    || ipv4_packet.src_addr() != ping_addr
                    || ipv4_packet.dst_addr() != *interface.ipv4_addr
                {
                    return Err(Error::Ignored);
                }

                let icmp_packet = Icmpv4Packet::try_new(ipv4_packet.payload())?;
                icmp_packet.check_encoding()?;
                let icmp_repr = Icmpv4Repr::deserialize(&icmp_packet)?;

                match icmp_repr.message {
                    Icmpv4Message::EchoReply {
                        id: id_reply,
                        seq: seq_reply,
                    } => {
                        if id_reply == id && seq_reply == seq && icmp_packet.payload() == payload {
                            Ok(())
                        } else {
                            Err(Error::Ignored)
                        }
                    }
                    _ => Err(Error::Ignored),
                }
            }) {
            return Some(waiting);
        }

        env::tick(interface, socket_set);
    }
}
