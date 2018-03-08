use {
    Error,
    Result,
};
use core::layers::{
    eth_types,
    EthernetFrame,
};
use core::services::{
    arp,
    Interface,
    ipv4,
};
use core::socket::{
    Packet,
    Socket,
    SocketSet,
};

/// Send an Ethernet frame via an interface.
pub fn send_frame<F>(interface: &mut Interface, eth_frame_len: usize, f: F) -> Result<()>
where
    F: FnOnce(&mut EthernetFrame<&mut [u8]>),
{
    let mut eth_buffer = vec![0; eth_frame_len];
    let mut eth_frame = EthernetFrame::try_new(&mut eth_buffer[..])?;
    f(&mut eth_frame);
    eth_frame.set_src_addr(interface.dev.ethernet_addr());
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
    sockets: &mut SocketSet,
) -> Result<()> {
    let eth_frame = EthernetFrame::try_new(eth_buffer)?;

    if eth_frame.dst_addr() != interface.dev.ethernet_addr() && !eth_frame.dst_addr().is_broadcast()
    {
        debug!(
            "Ignoring ethernet frame with destination {}.",
            eth_frame.dst_addr()
        );
        return Err(Error::NoOp);
    }

    for socket in sockets.iter_mut() {
        let packet = Packet::Raw(eth_frame.as_ref());
        match socket.recv_forward(&packet) {
            _ => {}
        }
    }

    match eth_frame.payload_type() {
        eth_types::ARP => arp::recv_packet(interface, eth_frame.payload()),
        eth_types::IPV4 => ipv4::recv_packet(interface, eth_frame.payload(), sockets),
        i => {
            debug!("Ignoring ethernet frame with type {}.", i);
            Err(Error::NoOp)
        }
    }
}
