use core::repr::Ipv4Packet;
use core::service::{
    ethernet,
    ipv4,
    tcp,
    udp,
    Interface,
};
use core::socket::{
    RawSocket,
    RawType,
    SocketSet,
    TaggedSocket,
    TcpSocket,
    UdpSocket,
};
use {
    Error,
    Result,
};

/// Sends out as many socket enqueued packets as possible via an interface.
pub fn send(interface: &mut Interface, socket_set: &mut SocketSet) {
    // Iterate over the sockets in round robin fashion (to avoid starvation) and
    // try to send a packet for each socket. Stop sending packets once we encounter
    // an error for each socket. This implies either (1) all the sockets have been
    // exhausted or (2) the device is busy.
    loop {
        let sockets = socket_set.count();
        let mut errors = 0;

        for socket in socket_set.iter_mut() {
            let ok_or_err = match *socket {
                TaggedSocket::Raw(ref mut socket) => send_raw_socket(interface, socket),
                TaggedSocket::Tcp(ref mut socket) => send_tcp_socket(interface, socket),
                TaggedSocket::Udp(ref mut socket) => send_udp_socket(interface, socket),
            };

            match ok_or_err {
                Ok(_) => {}
                Err(Error::Device(err)) => {
                    debug!(
                        "Device has encountered an error, probably exhausted {:?}.",
                        err
                    );
                    // Force exit from outer loop.
                    errors = sockets;
                    break;
                }
                Err(Error::Exhausted) => {
                    // These occur when the sockets are empty, let's not make our log useless
                    // with a flood of these errors.
                    errors += 1;
                }
                Err(err) => {
                    warn!("Error sending packet with {:?}.", err);
                    errors += 1;
                }
            }
        }

        if errors >= sockets {
            break;
        }
    }
}

fn send_raw_socket(interface: &mut Interface, socket: &mut RawSocket) -> Result<()> {
    match socket.raw_type() {
        RawType::Ethernet => {
            socket.send_dequeue(|eth_buffer| {
                ethernet::send_frame(interface, eth_buffer.len(), |eth_frame| {
                    // NOTE: We overwrite the MAC source address so the socket user should
                    // ensure this is set correctly in the frame they are writing.
                    eth_frame.as_mut().copy_from_slice(eth_buffer);
                })
            })
        }
        RawType::Ipv4 => socket.send_dequeue(|ipv4_buffer| {
            if let Ok(ipv4_packet) = Ipv4Packet::try_new(ipv4_buffer) {
                ipv4::send_packet_raw(
                    interface,
                    ipv4_packet.dst_addr(),
                    ipv4_buffer.len(),
                    |ipv4_packet| {
                        ipv4_packet.copy_from_slice(ipv4_buffer);
                    },
                )
            } else {
                warn!("Raw socket attempted to send a malformed IPv4 packet.");
                Ok(())
            }
        }),
    }
}

fn send_tcp_socket(interface: &mut Interface, socket: &mut TcpSocket) -> Result<()> {
    socket.send_dequeue(|ipv4_repr, tcp_repr, payload| {
        tcp::send_packet(interface, ipv4_repr, tcp_repr, |payload_| {
            payload_.copy_from_slice(payload);
        })
    })
}

fn send_udp_socket(interface: &mut Interface, socket: &mut UdpSocket) -> Result<()> {
    socket.send_dequeue(|ipv4_repr, udp_repr, payload| {
        udp::send_packet(interface, ipv4_repr, udp_repr, |payload_| {
            payload_.copy_from_slice(payload);
        })
    })
}

/// Reads frames from an interface and forwards packets to the appropriate
/// sockets.
pub fn recv(interface: &mut Interface, socket_set: &mut SocketSet) {
    let mut eth_buffer = vec![0; interface.dev.max_transmission_unit()];

    loop {
        let buffer_len = match interface.dev.recv(&mut eth_buffer) {
            Ok(buffer_len) => buffer_len,
            Err(Error::Device(_)) => break,
            Err(err) => {
                warn!("Error receiving Ethernet frame with {:?}.", err);
                break;
            }
        };

        match ethernet::recv_frame(interface, &eth_buffer[.. buffer_len], socket_set) {
            Ok(_) => continue,
            Err(Error::Ignored) => continue,
            Err(Error::MacResolution(_)) => continue,
            Err(err) => warn!("Error processing Ethernet frame with {:?}", err),
        }
    }
}
