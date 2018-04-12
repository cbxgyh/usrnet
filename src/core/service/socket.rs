use Error;
use core::repr::Ipv4Packet;
use core::service::{
    ethernet,
    tcp,
    udp,
    Interface,
    ipv4,
};
use core::socket::{
    Packet,
    Socket,
    SocketSet,
};

/// Sends out any packets enqueued in the sockets via an interface.
pub fn send(interface: &mut Interface, socket_set: &mut SocketSet) {
    for socket in socket_set.iter_mut() {
        loop {
            let ok_or_err = socket.send_forward(|packet| {
                match packet {
                    Packet::Raw(ref eth_buffer) => {
                        ethernet::send_frame(interface, eth_buffer.len(), |eth_frame| {
                            // NOTE: We overwrite the MAC source address so the socket user should
                            // ensure this is set correctly in the frame they are writing.
                            eth_frame.as_mut().copy_from_slice(eth_buffer);
                        })
                    }
                    Packet::Ipv4(ref ipv4_buffer) => {
                        if let Ok(ipv4_packet) = Ipv4Packet::try_new(ipv4_buffer) {
                            let ipv4_packet_len = ipv4_packet.as_ref().len();
                            ipv4::send_packet_raw(
                                interface,
                                ipv4_packet.dst_addr(),
                                ipv4_packet_len,
                                |ipv4_buffer| {
                                    ipv4_buffer.copy_from_slice(ipv4_packet.as_ref());
                                },
                            )
                        } else {
                            Ok(())
                        }
                    }
                    Packet::Tcp((ref ipv4_repr, ref tcp_repr, ref payload)) => {
                        tcp::send_packet(interface, ipv4_repr, tcp_repr, |payload_| {
                            payload_.copy_from_slice(payload);
                        })
                    }
                    Packet::Udp(ref ipv4_repr, ref udp_repr, ref payload) => {
                        udp::send_packet(interface, ipv4_repr, udp_repr, |payload_| {
                            payload_.copy_from_slice(payload);
                        })
                    }
                    _ => Err(Error::NoOp),
                }
            });

            match ok_or_err {
                Ok(_) => continue,
                Err(Error::Exhausted) => break,
                Err(err) => {
                    debug!("Error sending packet with {:?}.", err);
                    break;
                }
            }
        }
    }
}

/// Reads frames from an interface and forwards packets to the appropriate
/// sockets.
pub fn recv(interface: &mut Interface, socket_set: &mut SocketSet) {
    let mut eth_buffer = vec![0; interface.dev.max_transmission_unit()];

    loop {
        match interface.dev.recv(&mut eth_buffer) {
            Ok(buffer_len) => {
                match ethernet::recv_frame(interface, &mut eth_buffer[.. buffer_len], socket_set) {
                    Ok(_) => continue,
                    Err(Error::Address) => continue,
                    Err(Error::NoOp) => continue,
                    Err(err) => warn!("Error processing ethernet with {:?}", err),
                }
            }
            Err(Error::Exhausted) => break,
            Err(err) => warn!("Error receiving ethernet with {:?}", err),
        };
    }
}
