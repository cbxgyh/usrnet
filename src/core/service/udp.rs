use Result;
use core::repr::{
    Icmpv4DestinationUnreachable,
    Icmpv4Message,
    Icmpv4Repr,
    Ipv4Packet,
    Ipv4Protocol,
    Ipv4Repr,
    UdpPacket,
    UdpRepr,
};
use core::service::{
    Interface,
    icmpv4,
    ipv4,
};
use core::socket::{
    SocketAddr,
    SocketSet,
    TaggedSocket,
};

/// Sends a UDP packet via ther interface.
///
/// This function takes care of serializing a header, calculating a checksum,
/// etc. so the caller needs to fill in **only** the payload in the provided buffer.
pub fn send_packet<F>(
    interface: &mut Interface,
    ipv4_repr: &Ipv4Repr,
    udp_repr: &UdpRepr,
    f: F,
) -> Result<()>
where
    F: FnOnce(&mut [u8]),
{
    ipv4::send_packet_with_repr(interface, ipv4_repr, |ipv4_payload| {
        let mut udp_packet = UdpPacket::try_new(ipv4_payload).unwrap();
        f(udp_packet.payload_mut());
        // NOTE: It's important that the UDP serialization happens after the payload
        // is written to ensure a correct checksum.
        udp_repr.serialize(&mut udp_packet, ipv4_repr);
    })
}

/// Receives a UDP packet from an interface.
///
/// The UDP packet is parsed, forwarded to any socket, and any necessary ICMP
/// messages sent.
pub fn recv_packet(
    interface: &mut Interface,
    ipv4_repr: &Ipv4Repr,
    ipv4_packet: &Ipv4Packet<&[u8]>,
    socket_set: &mut SocketSet,
) -> Result<()> {
    let udp_packet = UdpPacket::try_new(ipv4_packet.payload())?;
    udp_packet.check_encoding(ipv4_repr)?;

    let udp_repr = UdpRepr::deserialize(&udp_packet);

    let dst_socket_addr = SocketAddr {
        addr: ipv4_repr.dst_addr,
        port: udp_repr.dst_port,
    };
    let mut unreachable = true;

    socket_set
        .iter_mut()
        .filter_map(|socket| match *socket {
            TaggedSocket::Udp(ref mut socket) => if socket.accepts(&dst_socket_addr) {
                Some(socket)
            } else {
                None
            },
            _ => None,
        })
        .for_each(|socket| {
            unreachable = false;
            if let Err(err) = socket.recv_enqueue(ipv4_repr, &udp_repr, udp_packet.payload()) {
                debug!(
                    "Error enqueueing UDP packet for receiving via socket with {:?}.",
                    err
                );
            }
        });

    // Send an ICMP message indicating packet has been ignored because no
    // UDP sockets are bound to the specified port.
    if unreachable {
        let icmp_repr = Icmpv4Repr {
            message: Icmpv4Message::DestinationUnreachable(
                Icmpv4DestinationUnreachable::PortUnreachable,
            ),
            payload_len: 28, // IP header (20 bytes) + UDP header (8 bytes)
        };
        let ipv4_repr = Ipv4Repr {
            src_addr: *interface.ipv4_addr,
            dst_addr: ipv4_repr.src_addr,
            protocol: Ipv4Protocol::ICMP,
            payload_len: icmp_repr.buffer_len() as u16,
        };
        debug!(
            "Sending ICMP {:?} in response to a UDP {:?}.",
            icmp_repr, udp_repr
        );
        icmpv4::send_packet(interface, &ipv4_repr, &icmp_repr, |payload| {
            let copy_len = payload.len() as usize;
            payload.copy_from_slice(&ipv4_packet.as_ref()[.. copy_len]);
        })
    } else {
        Ok(())
    }
}
