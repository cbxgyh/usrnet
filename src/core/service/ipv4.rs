use core::repr::{
    eth_types,
    ipv4_protocols,
    EthernetFrame,
    Ipv4Address,
    Ipv4Packet,
    Ipv4Repr,
};
use core::service::{
    arp,
    ethernet,
    icmpv4,
    tcp,
    udp,
    Interface,
};
use core::socket::{
    RawType,
    SocketSet,
    TaggedSocket,
};
use {
    Error,
    Result,
};

/// Send a raw IPv4 packet via the interface.
///
/// The appropriate Ethernet destination address will be inferred by the
/// network stack, but the callers is responsible for writing an entire well
/// formatted IPv4 packets to the provided buffer, NOT just the payload!
pub fn send_packet_raw<F>(
    interface: &mut Interface,
    dst_addr: Ipv4Address,
    ipv4_packet_len: usize,
    f: F,
) -> Result<()>
where
    F: FnOnce(&mut [u8]),
{
    let dst_addr = ipv4_addr_route(interface, dst_addr);
    let eth_dst_addr = arp::eth_addr_for_ip(interface, dst_addr)?;
    let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(ipv4_packet_len);

    ethernet::send_frame(interface, eth_frame_len, |eth_frame| {
        eth_frame.set_dst_addr(eth_dst_addr);
        eth_frame.set_payload_type(eth_types::IPV4);
        f(eth_frame.payload_mut());
    })
}

/// Sends an IPv4 packet via ther interface.
///
/// This is a "safe" version of send_packet_raw(...) which takes care of
/// serializing a header, calculating a checksum, etc. so the caller needs to
/// fill in **only** the payload in the provided buffer.
pub fn send_packet_with_repr<F>(interface: &mut Interface, ipv4_repr: &Ipv4Repr, f: F) -> Result<()>
where
    F: FnOnce(&mut [u8]),
{
    let (dst_addr, ipv4_packet_len) = (ipv4_repr.dst_addr, ipv4_repr.buffer_len());

    send_packet_raw(interface, dst_addr, ipv4_packet_len, |ipv4_buffer| {
        let mut ipv4_packet = Ipv4Packet::try_new(ipv4_buffer).unwrap();
        // NOTE: It's important to serialize the Ipv4Repr prior to calling payload_mut()
        // to ensure the header length is written and used when finding where the
        // payload is located in the packet!
        ipv4_repr.serialize(&mut ipv4_packet);
        f(ipv4_packet.payload_mut());
    })
}

/// Receives an ICMP packet from an interface.
///
/// The IPv4 packet is parsed, forwarded to any sockets, and propagated up the
/// network stack.
pub fn recv_packet(
    interface: &mut Interface,
    eth_frame: &EthernetFrame<&[u8]>,
    socket_set: &mut SocketSet,
) -> Result<()> {
    let ipv4_packet = Ipv4Packet::try_new(eth_frame.payload())?;
    ipv4_packet.check_encoding()?;

    if ipv4_packet.dst_addr() != *interface.ipv4_addr {
        debug!(
            "Ignoring IPv4 packet with destination {}.",
            ipv4_packet.dst_addr()
        );
        return Err(Error::Ignored);
    }

    // Update ARP cache! This is important for generating IMMEDIATE (not socket
    // buffered) ICMP echo replies, errors, etc.
    if eth_frame.src_addr().is_unicast() {
        interface
            .arp_cache
            .set_eth_addr_for_ip(ipv4_packet.src_addr(), eth_frame.src_addr());
    }

    socket_set
        .iter_mut()
        .filter_map(|socket| match *socket {
            TaggedSocket::Raw(ref mut socket) => if socket.raw_type() == RawType::Ipv4 {
                Some(socket)
            } else {
                None
            },
            _ => None,
        })
        .for_each(|socket| {
            if let Err(err) = socket.recv_enqueue(ipv4_packet.as_ref()) {
                debug!(
                    "Error enqueueing IPv4 packet for receiving via socket with {:?}.",
                    err
                );
            }
        });

    let ipv4_repr = Ipv4Repr::deserialize(&ipv4_packet)?;

    match ipv4_packet.protocol() {
        ipv4_protocols::TCP => tcp::recv_packet(interface, &ipv4_repr, &ipv4_packet, socket_set),
        ipv4_protocols::UDP => udp::recv_packet(interface, &ipv4_repr, &ipv4_packet, socket_set),
        ipv4_protocols::ICMP => icmpv4::recv_packet(interface, &ipv4_repr, ipv4_packet.payload()),
        i => {
            debug!("Ignoring IPv4 packet with type {}.", i);
            Err(Error::Ignored)
        }
    }
}

/// Returns the next hop for a packet destined to a specified address.
pub fn ipv4_addr_route(interface: &mut Interface, address: Ipv4Address) -> Ipv4Address {
    if interface.ipv4_addr.is_member(address) {
        debug!("{} will be routed through link.", address);
        address
    } else {
        debug!("{} will be routed through default gateway.", address);
        interface.default_gateway
    }
}
