use std::time::{
    Duration,
    Instant,
};

use rand;

use Error;
use core::repr::{
    Icmpv4DestinationUnreachable,
    Icmpv4Packet,
    Icmpv4Repr,
    Icmpv4TimeExceeded,
    Ipv4Address,
    Ipv4Packet,
    Ipv4Protocol,
    Ipv4Repr,
    UdpPacket,
    UdpRepr,
    ipv4_protocols,
};
use core::service::Interface;
use core::socket::{
    SocketAddr,
    SocketSet,
};
use examples::env;

const PORT_MIN: u16 = 33434;

const PORT_MAX: u16 = 33534;

/// Performs a traceroute via UDP packets.
///
/// Up until the max TTL is reached (starting at 1) or we receive a reply from
/// the specified address, the following loop is performed.
///
/// 1. Send a UDP packet on a random port in the range [33434, 33534].
///
/// 2. Wait for an ICMP Time Exceeded or Destination Unreachable response until
///    the specified timeout.
pub fn traceroute<F>(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    raw_handle: usize,
    addr: Ipv4Address,
    payload_len: usize,
    max_ttl: u8,
    timeout: Duration,
    mut f: F,
) -> Option<()>
where
    F: FnMut(u8, Option<(Duration, Ipv4Address)>),
{
    // Send UDP packet to a random port.
    let port = PORT_MIN + rand::random::<u16>() % (PORT_MAX - PORT_MIN + 1);
    let socket_addr = SocketAddr { addr, port };

    for ttl in 1 .. (max_ttl + 1) {
        send(
            interface,
            socket_set,
            raw_handle,
            socket_addr,
            payload_len,
            ttl,
        );
        let response = recv(interface, socket_set, raw_handle, socket_addr, timeout);
        f(ttl, response);
        if let Some((_, endpoint)) = response {
            if endpoint == addr {
                return Some(());
            }
        }
    }

    None
}

/// Sends a UDP packet to the specified (addr, port).
///
/// The UDP will be enqueued on a socket, not necessarily forwarded onto the link.
fn send(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    raw_handle: usize,
    socket_addr: SocketAddr,
    payload_len: usize,
    ttl: u8,
) {
    // Assuming 5 word/20 byte IP header!
    let udp_repr = UdpRepr {
        src_port: socket_addr.port,
        dst_port: socket_addr.port,
        length: (8 + payload_len) as u16,
    };

    let ipv4_repr = Ipv4Repr {
        src_addr: *interface.ipv4_addr,
        dst_addr: socket_addr.addr,
        protocol: Ipv4Protocol::UDP,
        payload_len: udp_repr.buffer_len() as u16,
    };

    // Socket may have a full send buffer!
    while let Err(_) = socket_set
        .socket(raw_handle)
        .as_raw_socket()
        .send(ipv4_repr.buffer_len())
        .map(|ip_buffer| {
            let mut ipv4_packet = Ipv4Packet::try_new(ip_buffer).unwrap();
            ipv4_repr.serialize(&mut ipv4_packet);

            // We need to update the checksum manually if we set a custom TTL,
            // or any header field.
            ipv4_packet.set_ttl(ttl as u8);
            ipv4_packet.set_header_checksum(0);
            let checksum = ipv4_packet.gen_header_checksum();
            ipv4_packet.set_header_checksum(checksum);

            let mut udp_packet = UdpPacket::try_new(ipv4_packet.payload_mut()).unwrap();
            for i in 0 .. payload_len {
                udp_packet.payload_mut()[i] = rand::random::<u8>();
            }
            udp_repr.serialize(&mut udp_packet, &ipv4_repr);
        }) {
        env::tick(interface, socket_set);
    }
}

/// Waits for a Time Exceeded or Destination Unreachable ICMP error in response to a UDP packet
/// up until the specified timeout.
fn recv(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    raw_handle: usize,
    socket_addr: SocketAddr,
    timeout: Duration,
) -> Option<(Duration, Ipv4Address)> {
    let wait_at = Instant::now();

    loop {
        let waiting = Instant::now().duration_since(wait_at);

        if waiting >= timeout {
            return None;
        } else if let Ok(response_addr) = socket_set
            .socket(raw_handle)
            .as_raw_socket()
            .recv()
            .and_then(|ip_buffer| {
                let ipv4_packet = Ipv4Packet::try_new(ip_buffer)?;
                if ipv4_packet.protocol() != ipv4_protocols::ICMP
                    || ipv4_packet.dst_addr() != *interface.ipv4_addr
                {
                    return Err(Error::NoOp);
                }

                let response_addr = ipv4_packet.src_addr();

                // We care only about two cases of ICMP messages:
                //
                // 1. Destination Unreachable => If the UDP packet reached the final host.
                // 2. Time Exceeded           => If the UDP packet was dropped by a router.
                let icmp_packet = Icmpv4Packet::try_new(ipv4_packet.payload())?;
                icmp_packet.check_encoding()?;
                let icmp_repr = Icmpv4Repr::deserialize(&icmp_packet)?;
                let ipv4_packet = match icmp_repr {
                    Icmpv4Repr::DestinationUnreachable {
                        reason: Icmpv4DestinationUnreachable::PortUnreachable,
                        ..
                    } => Ipv4Packet::try_new(icmp_packet.payload())?,
                    Icmpv4Repr::TimeExceeded {
                        reason: Icmpv4TimeExceeded::TTLExpired,
                        ..
                    } => Ipv4Packet::try_new(icmp_packet.payload())?,
                    _ => return Err(Error::NoOp),
                };

                // So I'm not 100% sure about this, but let's check the (1) destination address
                // and (2) transport protocol only since source address, checksum, etc. can get
                // modified by a NAT.
                if ipv4_packet.dst_addr() != socket_addr.addr
                    || ipv4_packet.protocol() != ipv4_protocols::UDP
                {
                    return Err(Error::NoOp);
                }

                // We only have a portion of the original IP packet, so let's be careful parsing
                // the payload...
                let ip_header_len = (ipv4_packet.header_len() * 4) as usize;
                let ip_payload = &ipv4_packet.as_ref()[ip_header_len ..];
                let udp_packet = UdpPacket::try_new(ip_payload)?;

                // Likewise, let's inspect the destination port only since the source port might
                // have gotten modified by a NAT.
                if udp_packet.dst_port() != socket_addr.port {
                    Err(Error::NoOp)
                } else {
                    Ok(response_addr)
                }
            }) {
            return Some((waiting, response_addr));
        }

        env::tick(interface, socket_set);
    }
}
