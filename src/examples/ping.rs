use std::time::{
    Duration,
    Instant,
};

use Error;
use core::repr::{
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    Ipv4Packet,
    Ipv4Protocol,
    Ipv4Repr,
    ipv4_protocols,
};
use core::service::Interface;
use core::socket::SocketSet;
use examples::env;

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
    let icmp_repr = Icmpv4Repr::EchoRequest { id, seq };

    let ip_repr = Ipv4Repr {
        src_addr: *interface.ipv4_addr,
        dst_addr: ping_addr,
        protocol: Ipv4Protocol::ICMP,
        payload_len: (icmp_repr.buffer_len() + payload.len()) as u16,
    };

    // Socket may have a full send buffer!
    while let Err(_) = socket_set
        .socket(raw_handle)
        .as_raw_socket()
        .send(ip_repr.buffer_len())
        .map(|ip_buffer| {
            let mut ip_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
            ip_repr.serialize(&mut ip_packet);

            let mut icmp_packet = Icmpv4Packet::try_new(ip_packet.payload_mut()).unwrap();
            icmp_packet.payload_mut().copy_from_slice(payload);
            icmp_repr.serialize(&mut icmp_packet).unwrap();
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
                let ip_packet = Ipv4Packet::try_new(ip_buffer)?;
                if ip_packet.protocol() != ipv4_protocols::ICMP || ip_packet.src_addr() != ping_addr
                    || ip_packet.dst_addr() != *interface.ipv4_addr
                {
                    return Err(Error::NoOp);
                }

                let icmp_packet = Icmpv4Packet::try_new(ip_packet.payload())?;
                icmp_packet.check_encoding()?;
                let icmp_repr = Icmpv4Repr::deserialize(&icmp_packet)?;

                match icmp_repr {
                    Icmpv4Repr::EchoReply {
                        id: id_reply,
                        seq: seq_reply,
                    } => {
                        if id_reply == id && seq_reply == seq && icmp_packet.payload() == payload {
                            Ok(())
                        } else {
                            Err(Error::NoOp)
                        }
                    }
                    _ => Err(Error::NoOp),
                }
            }) {
            return Some(waiting);
        }

        env::tick(interface, socket_set);
    }
}
