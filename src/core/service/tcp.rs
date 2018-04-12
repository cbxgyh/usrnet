use Result;
use core::repr::{
    Ipv4Packet,
    Ipv4Repr,
    TcpPacket,
    TcpRepr,
};
use core::service::{
    Interface,
    ipv4,
};
use core::socket::{
    Packet,
    Socket,
    SocketSet,
};

/// Sends a TCP packet via the interface.
///
/// This function takes care of serializing a header, calculating a checksum,
/// etc. so the caller needs to fill in **only** the payload in the provided buffer.
pub fn send_packet<F>(
    interface: &mut Interface,
    ipv4_repr: &Ipv4Repr,
    tcp_repr: &TcpRepr,
    f: F,
) -> Result<()>
where
    F: FnOnce(&mut [u8]),
{
    ipv4::send_packet_with_repr(interface, ipv4_repr, |ipv4_payload| {
        let mut tcp_packet = TcpPacket::try_new(ipv4_payload).unwrap();
        tcp_repr.serialize(&mut tcp_packet).unwrap();
        f(tcp_packet.payload_mut());
        tcp_packet.fill_checksum(ipv4_repr);
    })
}

/// Receives a TCP packet from an interface.
///
/// The TCP packet is parsed, forwarded to any socket, and any necessary TCP
/// reset messages sent.
pub fn recv_packet(
    _interface: &mut Interface,
    ipv4_repr: &Ipv4Repr,
    ipv4_packet: &Ipv4Packet<&[u8]>,
    socket_set: &mut SocketSet,
) -> Result<()> {
    let tcp_packet = TcpPacket::try_new(ipv4_packet.payload())?;
    tcp_packet.check_encoding(ipv4_repr)?;

    let tcp_repr = TcpRepr::deserialize(&tcp_packet);

    let packet = Packet::Tcp((*ipv4_repr, tcp_repr, tcp_packet.payload()));
    for socket in socket_set.iter_mut() {
        socket.recv_forward(&packet).ok();
    }

    // TODO: Send RST message if SYN packet was not accepted by any sockets.
    Ok(())
}
