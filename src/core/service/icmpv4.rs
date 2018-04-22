use std::mem::swap;

use {
    Error,
    Result,
};
use core::repr::{
    Icmpv4Message,
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Repr,
};
use core::service::{
    Interface,
    ipv4,
};

/// Send an ICMP packet via the interface.
pub fn send_packet<F>(
    interface: &mut Interface,
    ipv4_repr: &Ipv4Repr,
    icmp_repr: &Icmpv4Repr,
    f: F,
) -> Result<()>
where
    F: FnOnce(&mut [u8]),
{
    ipv4::send_packet_with_repr(interface, &ipv4_repr, |ipv4_payload| {
        let mut icmp_packet = Icmpv4Packet::try_new(ipv4_payload).unwrap();
        icmp_repr.serialize(&mut icmp_packet).unwrap();
        f(icmp_packet.payload_mut());
        icmp_packet.fill_checksum();
    })
}

/// Receives an ICMP packet from an interface.
///
/// This may result in a response to ICMP echo requests, etc.
pub fn recv_packet(
    interface: &mut Interface,
    ipv4_repr: &Ipv4Repr,
    icmp_buffer: &[u8],
) -> Result<()> {
    let icmp_recv_packet = Icmpv4Packet::try_new(icmp_buffer)?;
    icmp_recv_packet.check_encoding()?;

    let icmp_recv_repr = Icmpv4Repr::deserialize(&icmp_recv_packet)?;

    let (ipv4_send_repr, icmp_send_repr) = match icmp_recv_repr.message {
        Icmpv4Message::EchoRequest { id, seq } => {
            debug!(
                "Got a ping from {}; Sending response...",
                ipv4_repr.src_addr
            );
            let mut ipv4_send_repr = ipv4_repr.clone();
            swap(&mut ipv4_send_repr.src_addr, &mut ipv4_send_repr.dst_addr);
            (
                ipv4_send_repr,
                Icmpv4Repr {
                    message: Icmpv4Message::EchoReply { id, seq },
                    payload_len: icmp_recv_repr.payload_len,
                },
            )
        }
        _ => return Err(Error::Ignored),
    };

    send_packet(interface, &ipv4_send_repr, &icmp_send_repr, |payload| {
        payload.copy_from_slice(icmp_recv_packet.payload());
    })
}
