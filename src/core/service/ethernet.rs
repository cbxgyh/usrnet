use core::repr::{
    eth_types,
    EthernetFrame,
};
use core::service::{
    arp,
    ipv4,
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

/// Send an Ethernet frame via an interface.
pub fn send_frame<F>(interface: &mut Interface, eth_frame_len: usize, f: F) -> Result<()>
where
    F: FnOnce(&mut EthernetFrame<&mut [u8]>),
{
    let mut eth_buffer = vec![0; eth_frame_len];
    let mut eth_frame = EthernetFrame::try_new(&mut eth_buffer[..])?;
    f(&mut eth_frame);
    eth_frame.set_src_addr(interface.ethernet_addr);
    interface.dev.send(eth_frame.as_ref())?;
    Ok(())
}

/// Receives an Ethernet frame from an interface.
///
/// The Ethernet frame is parsed, forwarded to any sockets, and propagated up
/// the network stack.
pub fn recv_frame(
    interface: &mut Interface,
    eth_buffer: &[u8],
    socket_set: &mut SocketSet,
) -> Result<()> {
    let eth_frame = EthernetFrame::try_new(eth_buffer)?;

    if eth_frame.dst_addr() != interface.ethernet_addr && !eth_frame.dst_addr().is_broadcast() {
        debug!(
            "Ignoring ethernet frame with destination {}.",
            eth_frame.dst_addr()
        );
        return Err(Error::Ignored);
    }

    socket_set
        .iter_mut()
        .filter_map(|socket| match *socket {
            TaggedSocket::Raw(ref mut socket) => if socket.raw_type() == RawType::Ethernet {
                Some(socket)
            } else {
                None
            },
            _ => None,
        })
        .for_each(|socket| {
            if let Err(err) = socket.recv_enqueue(eth_frame.as_ref()) {
                debug!(
                    "Error enqueueing Ethernet frame for receiving via socket with {:?}.",
                    err
                );
            }
        });

    match eth_frame.payload_type() {
        eth_types::ARP => arp::recv_packet(interface, &eth_frame),
        eth_types::IPV4 => ipv4::recv_packet(interface, &eth_frame, socket_set),
        i => {
            debug!("Ignoring ethernet frame with type {}.", i);
            Err(Error::Ignored)
        }
    }
}
