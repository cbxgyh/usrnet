use core::arp_cache::ArpCache;
use core::dev::{
    Device,
    Error as DevError,
};
use core::layers::{
    ethernet_types,
    Arp,
    ArpOp,
    EthernetAddress,
    EthernetFrame,
};

pub struct Service<D>
where
    D: Device,
{
    dev: D,
    arp_cache: ArpCache,
}

impl<D> Service<D>
where
    D: Device,
{
    pub fn new(dev: D, arp_cache: ArpCache) -> Service<D> {
        Service { dev, arp_cache }
    }
}

impl<D> Service<D>
where
    D: Device,
{
    /// Processes all ingress traffic on the associated device.
    pub fn recv(&mut self) {
        let mut recv_buffer = vec![0; self.dev.max_transmission_unit()];

        loop {
            match self.dev.recv(recv_buffer.as_mut()) {
                Ok(buffer_len) => self.recv_ethernet(&recv_buffer[..buffer_len]),
                Err(DevError::Nothing) => break,
                Err(err) => {
                    warn!("Device::recv(...) failed with {:?}.", err);
                    break;
                }
            };
        }
    }

    fn send_ethernet<F>(&mut self, buffer_len: usize, f: F)
    where
        F: FnOnce(&mut EthernetFrame<&mut [u8]>),
    {
        let frame_len = EthernetFrame::<&[u8]>::buffer_len(buffer_len);
        let mut buffer = vec![0; frame_len];

        {
            let mut eth_frame = EthernetFrame::try_from(&mut buffer[..]).unwrap();
            eth_frame.set_src_addr(self.dev.ethernet_addr());
            f(&mut eth_frame);
        }

        match self.dev.send(buffer.as_ref()) {
            Err(err) => warn!("Device::send(...) failed with {:?}.", err),
            _ => {}
        };
    }

    fn recv_ethernet(&mut self, eth_buffer: &[u8]) {
        match EthernetFrame::try_from(eth_buffer) {
            Ok(eth_frame) => {
                if eth_frame.dst_addr() != self.dev.ethernet_addr()
                    && eth_frame.dst_addr() != EthernetAddress::BROADCAST
                {
                    debug!(
                        "Service::recv_ethernet(...) ignoring frame with destination {}.",
                        eth_frame.dst_addr()
                    );
                }
                match eth_frame.payload_type() {
                    ethernet_types::ARP => self.recv_arp(eth_frame.payload()),
                    i => debug!(
                        "Service::recv_ethernet(...) ignoring frame with type {}.",
                        i
                    ),
                }
            }
            Err(err) => debug!(
                "EthernetFrame::try_from(..) failed on {:?} with {:?}.",
                eth_buffer, err
            ),
        }
    }

    fn recv_arp(&mut self, arp_packet: &[u8]) {
        match Arp::deserialize(arp_packet) {
            Ok(Arp::EthernetIpv4 {
                op,
                source_hw_addr,
                source_proto_addr,
                target_proto_addr,
                ..
            }) => {
                if op != ArpOp::Request || target_proto_addr != self.dev.ipv4_addr() {
                    debug!("Service::recv_arp(...) ignoring ARP {:?}.", arp_packet);
                    return;
                }

                let arp_repr = Arp::EthernetIpv4 {
                    op: ArpOp::Reply,
                    source_hw_addr: self.dev.ethernet_addr(),
                    source_proto_addr: self.dev.ipv4_addr(),
                    target_hw_addr: source_hw_addr,
                    target_proto_addr: source_proto_addr,
                };

                self.arp_cache
                    .set_eth_addr_for_ip(source_proto_addr, source_hw_addr);

                debug!(
                    "Service::recv_arp(...) sending ARP reply to {}/{}.",
                    source_proto_addr, source_hw_addr
                );

                self.send_ethernet(arp_repr.buffer_len(), |eth_frame| {
                    eth_frame.set_dst_addr(source_hw_addr);
                    eth_frame.set_payload_type(ethernet_types::ARP as u16);
                    arp_repr.serialize(eth_frame.payload_mut()).unwrap();
                });
            }
            Err(err) => debug!(
                "Arp::deserialize(...) failed on {:?} with {:?}.",
                arp_packet, err
            ),
        }
    }
}
