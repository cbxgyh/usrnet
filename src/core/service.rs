use std;

use core::arp_cache::ArpCache;
use core::dev::{
    Device,
    Error as DevError,
};
use core::layers::{
    ethernet_types,
    Arp,
    ArpOp,
    Error as LayerError,
    EthernetAddress,
    EthernetFrame,
    Ipv4Address,
    Ipv4Packet,
    ipv4_flags,
};

#[derive(Debug)]
pub enum Error {
    /// Device error.
    Device(DevError),
    /// Layer error.
    Layer(LayerError),
    /// Error resolving an IPv4 address.
    Address,
    /// Indicates a no-op.
    NoOp,
}

impl From<DevError> for Error {
    fn from(error: DevError) -> Self {
        Error::Device(error)
    }
}

impl From<LayerError> for Error {
    fn from(error: LayerError) -> Self {
        Error::Layer(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

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
                Ok(buffer_len) => match self.recv_ethernet(&recv_buffer[..buffer_len]) {
                    Ok(_) => continue,
                    Err(Error::Layer(err)) => warn!("Layer failed with {:?}", err),
                    Err(Error::Device(err)) => warn!("Device failed with {:?}", err),
                    Err(Error::Address) => continue,
                    Err(Error::NoOp) => continue,
                },
                Err(DevError::Nothing) => break,
                Err(err) => warn!("Device failed with {:?}", err),
            };
        }
    }

    pub fn send_ipv4_packet<F>(
        &mut self,
        buffer_len: usize,
        ipv4_dst_addr: Ipv4Address,
        f: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut Ipv4Packet<&mut [u8]>),
    {
        let eth_dst_addr = self.eth_addr_for_ip(ipv4_dst_addr)?;
        let src_ip_addr = self.dev.ipv4_addr();
        let buffer_len = Ipv4Packet::<&[u8]>::buffer_len(buffer_len);

        self.send_eth_frame(buffer_len, |eth_frame| {
            eth_frame.set_dst_addr(eth_dst_addr);
            eth_frame.set_payload_type(ethernet_types::IPV4);

            let mut ip_packet = Ipv4Packet::try_from(eth_frame.payload_mut()).unwrap();
            ip_packet.set_ip_version(4);
            ip_packet.set_header_len(5);
            ip_packet.set_dscp(0);
            ip_packet.set_ecn(0);
            ip_packet.set_packet_len(buffer_len as u16);
            ip_packet.set_identification(0);
            ip_packet.set_flags(ipv4_flags::DONT_FRAGMENT);
            ip_packet.set_fragment_offset(0);
            ip_packet.set_ttl(64);
            ip_packet.set_header_checksum(0);
            ip_packet.set_src_addr(src_ip_addr);

            f(&mut ip_packet);

            let header_checksum = ip_packet.gen_header_checksum();
            ip_packet.set_header_checksum(header_checksum);
        })
    }

    pub fn send_eth_frame<F>(&mut self, buffer_len: usize, f: F) -> Result<()>
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

        let _ = self.dev.send(buffer.as_ref())?;
        Ok(())
    }

    fn recv_ethernet(&mut self, eth_buffer: &[u8]) -> Result<()> {
        let eth_frame = EthernetFrame::try_from(eth_buffer)?;
        if eth_frame.dst_addr() != self.dev.ethernet_addr()
            && eth_frame.dst_addr() != EthernetAddress::BROADCAST
        {
            debug!(
                "Ignoring ethernet frame with destination {}.",
                eth_frame.dst_addr()
            );
        }
        match eth_frame.payload_type() {
            ethernet_types::ARP => self.recv_arp_packet(eth_frame.payload()),
            i => {
                debug!("Ignoring ethernet frame with type {}.", i);
                Err(Error::NoOp)
            }
        }
    }

    fn recv_arp_packet(&mut self, arp_packet: &[u8]) -> Result<()> {
        let arp_repr = Arp::deserialize(arp_packet)?;
        match arp_repr {
            Arp::EthernetIpv4 {
                op,
                source_hw_addr,
                source_proto_addr,
                target_proto_addr,
                ..
            } => {
                if target_proto_addr != self.dev.ipv4_addr() {
                    debug!(
                        "Ignoring ARP with target IPv4 address {}.",
                        target_proto_addr
                    );
                    return Err(Error::NoOp);
                }

                self.arp_cache
                    .set_eth_addr_for_ip(source_proto_addr, source_hw_addr);

                match op {
                    ArpOp::Request => {
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
                            "Sending ARP reply to {}/{}.",
                            source_proto_addr, source_hw_addr
                        );

                        self.send_eth_frame(arp_repr.buffer_len(), |eth_frame| {
                            eth_frame.set_dst_addr(source_hw_addr);
                            eth_frame.set_payload_type(ethernet_types::ARP);
                            arp_repr.serialize(eth_frame.payload_mut()).unwrap();
                        })
                    }
                    _ => Ok(()),
                }
            }
        }
    }

    fn eth_addr_for_ip(&mut self, ipv4_addr: Ipv4Address) -> Result<EthernetAddress> {
        match self.arp_cache.eth_addr_for_ip(ipv4_addr) {
            Some(eth_addr) => Ok(eth_addr),
            None => {
                let arp_repr = Arp::EthernetIpv4 {
                    op: ArpOp::Request,
                    source_hw_addr: self.dev.ethernet_addr(),
                    source_proto_addr: self.dev.ipv4_addr(),
                    target_hw_addr: EthernetAddress::BROADCAST,
                    target_proto_addr: ipv4_addr,
                };

                debug!("Sending ARP request for {}.", ipv4_addr);

                self.send_eth_frame(arp_repr.buffer_len(), |eth_frame| {
                    eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
                    eth_frame.set_payload_type(ethernet_types::ARP);
                    arp_repr.serialize(eth_frame.payload_mut()).unwrap();
                })?;

                Err(Error::Address)
            }
        }
    }
}
