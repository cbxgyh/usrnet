use std::time::{
    Duration,
    Instant,
};

use core::repr::{
    eth_types,
    Arp,
    ArpOp,
    EthernetAddress,
    EthernetFrame,
    Ipv4Address,
};
use core::service::Interface;
use core::socket::SocketSet;
use examples::env;
use Error;

/// Sends an ARP request for an IP address via a raw Ethernet socket.
pub fn arping(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    raw_handle: usize,
    arping_addr: Ipv4Address,
    timeout: Duration,
) -> Option<(Duration, EthernetAddress)> {
    let arp_repr = Arp {
        op: ArpOp::Request,
        source_hw_addr: interface.ethernet_addr,
        source_proto_addr: *interface.ipv4_addr,
        target_hw_addr: EthernetAddress::BROADCAST,
        target_proto_addr: arping_addr,
    };

    let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(arp_repr.buffer_len());

    // Socket may have a full send buffer!
    while let Err(_) = socket_set
        .socket(raw_handle)
        .as_raw_socket()
        .send(eth_frame_len)
        .map(|eth_buffer| {
            let mut eth_frame = EthernetFrame::try_new(eth_buffer).unwrap();
            eth_frame.set_src_addr(interface.ethernet_addr);
            eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
            eth_frame.set_payload_type(eth_types::ARP);
            arp_repr.serialize(eth_frame.payload_mut()).unwrap();
        }) {
        env::tick(interface, socket_set);
    }

    let send_at = Instant::now();

    loop {
        let waiting = Instant::now().duration_since(send_at);

        if waiting >= timeout {
            return None;
        } else if let Ok(eth_addr) = socket_set
            .socket(raw_handle)
            .as_raw_socket()
            .recv()
            .and_then(|eth_buffer| {
                let eth_frame = EthernetFrame::try_new(eth_buffer)?;
                if eth_frame.payload_type() != eth_types::ARP {
                    return Err(Error::Ignored);
                }

                let arp_repr = Arp::deserialize(eth_frame.payload())?;
                if arp_repr.op == ArpOp::Reply && arp_repr.source_proto_addr == arping_addr {
                    Ok(arp_repr.source_hw_addr)
                } else {
                    Err(Error::Ignored)
                }
            }) {
            return Some((waiting, eth_addr));
        }

        env::tick(interface, socket_set);
    }
}
