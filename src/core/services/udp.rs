use Result;
use core::repr::{
    Icmpv4DestinationUnreachable,
    Icmpv4Repr,
    Ipv4Packet,
    Ipv4Protocol,
    Ipv4Repr,
    UdpPacket,
    UdpRepr,
};
use core::services::{
    Interface,
    icmpv4,
    ipv4,
};
use core::socket::{
    Packet,
    Socket,
    SocketSet,
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
    sockets: &mut SocketSet,
) -> Result<()> {
    let udp_packet = UdpPacket::try_new(ipv4_packet.payload())?;
    udp_packet.check_encoding(ipv4_repr)?;

    let udp_repr = UdpRepr::deserialize(&udp_packet)?;

    let packet = Packet::Udp(*ipv4_repr, udp_repr, udp_packet.payload());
    let mut unreachable = true;
    for socket in sockets.iter_mut() {
        match socket.recv_forward(&packet) {
            Ok(_) => unreachable = false,
            _ => {}
        }
    }

    // Send an ICMP message indicating packet has been ignored because no
    // UDP sockets are bound to the specified port.
    if unreachable {
        let icmp_repr = Icmpv4Repr::DestinationUnreachable {
            reason: Icmpv4DestinationUnreachable::PortUnreachable,
            ipv4_header_len: (ipv4_packet.header_len() * 4) as usize,
        };
        let ipv4_repr = Ipv4Repr {
            src_addr: *interface.dev.ipv4_addr(),
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
