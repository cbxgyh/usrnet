use std::mem::swap;

use {
    Error,
    Result,
};
use core::arp_cache::ArpCache;
use core::dev::Device;
use core::layers::{
    eth_types,
    Arp,
    ArpOp,
    EthernetAddress,
    EthernetFrame,
    Icmpv4DestinationUnreachable,
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    Ipv4Packet,
    Ipv4Protocol,
    Ipv4Repr,
    UdpPacket,
    UdpRepr,
    ipv4_protocols,
};
use core::socket::{
    Packet,
    Socket,
    SocketSet,
};

pub struct Service<D: Device> {
    dev: D,
    arp_cache: ArpCache,
    default_gateway: Ipv4Address,
}

impl<D: Device> Service<D> {
    pub fn new(dev: D, arp_cache: ArpCache, default_gateway: Ipv4Address) -> Service<D> {
        Service {
            dev,
            arp_cache,
            default_gateway,
        }
    }
}

impl<D: Device> Service<D> {
    /// Sends out all egress traffic on the provided sockets.
    pub fn send(&mut self, sockets: &mut SocketSet) {
        for socket in sockets.iter_mut() {
            loop {
                match socket.send_forward(|packet| match packet {
                    Packet::Raw(ref eth_buffer) => {
                        self.send_eth_frame(eth_buffer.len(), |eth_frame| {
                            // NOTE: We overwrite the MAC source address so the socket user should
                            // ensure this is set correctly in the frame they are writing.
                            eth_frame.as_mut().copy_from_slice(eth_buffer);
                        })
                    }
                    Packet::Ipv4(ref ipv4_buffer) => {
                        if let Ok(ipv4_packet) = Ipv4Packet::try_new(ipv4_buffer) {
                            let ipv4_packet_len = ipv4_packet.as_ref().len();
                            self.send_ipv4_packet_raw(
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
                    Packet::Udp(ref ipv4_repr, ref udp_repr, ref payload) => {
                        self.send_udp_packet(ipv4_repr, udp_repr, |payload_| {
                            payload_.copy_from_slice(payload);
                        })
                    }
                    _ => Err(Error::NoOp),
                }) {
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

    /// Processes all ingress traffic on the associated device and forward
    /// packets to the appropriate sockets.
    pub fn recv(&mut self, sockets: &mut SocketSet) {
        let mut eth_buffer = vec![0; self.dev.max_transmission_unit()];

        loop {
            match self.dev.recv(&mut eth_buffer) {
                Ok(buffer_len) => {
                    match self.recv_eth_frame(&mut eth_buffer[.. buffer_len], sockets) {
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

    fn send_udp_packet<F>(&mut self, ipv4_repr: &Ipv4Repr, udp_repr: &UdpRepr, f: F) -> Result<()>
    where
        F: FnOnce(&mut [u8]),
    {
        self.send_ipv4_packet_with_repr(ipv4_repr, |ipv4_payload| {
            let mut udp_packet = UdpPacket::try_new(ipv4_payload).unwrap();
            f(udp_packet.payload_mut());
            // NOTE: It's important that the UDP serialization happens after the payload
            // is written to ensure a correct checksum.
            udp_repr.serialize(&mut udp_packet, ipv4_repr);
        })
    }

    fn recv_udp_packet(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        ipv4_packet: &Ipv4Packet<&[u8]>,
        sockets: &mut SocketSet,
    ) -> Result<()> {
        let udp_packet = UdpPacket::try_new(ipv4_packet.payload())?;
        udp_packet.check_encoding(ipv4_repr)?;

        let udp_repr = UdpRepr::deserialize(&udp_packet)?;

        let packet = Packet::Udp(*ipv4_repr, udp_repr, udp_packet.payload());
        let mut unreachable = true;
        for socket in sockets.iter_mut() {
            match socket.recv_forward(&packet) {
                Ok(_) => unreachable = false,
                _ => {}
            }
        }

        // Send an ICMP message indicating packet has been ignored because no
        // UDP sockets are bound to the specified port.
        if unreachable {
            let icmp_repr = Icmpv4Repr::DestinationUnreachable {
                reason: Icmpv4DestinationUnreachable::PortUnreachable,
                ipv4_header_len: (ipv4_packet.header_len() * 4) as usize,
            };
            let ipv4_repr = Ipv4Repr {
                src_addr: *self.dev.ipv4_addr(),
                dst_addr: ipv4_repr.src_addr,
                protocol: Ipv4Protocol::ICMP,
                payload_len: icmp_repr.buffer_len() as u16,
            };
            debug!(
                "Sending ICMP {:?} in response to a UDP {:?}.",
                icmp_repr, udp_repr
            );
            self.send_icmp_packet(&ipv4_repr, &icmp_repr, |payload| {
                let copy_len = payload.len() as usize;
                payload.copy_from_slice(&ipv4_packet.as_ref()[.. copy_len]);
            })
        } else {
            Ok(())
        }
    }

    fn send_icmp_packet<F>(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        icmp_repr: &Icmpv4Repr,
        f: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut [u8]),
    {
        self.send_ipv4_packet_with_repr(&ipv4_repr, |ipv4_payload| {
            let mut icmp_packet = Icmpv4Packet::try_new(ipv4_payload).unwrap();
            f(icmp_packet.payload_mut());
            // NOTE: It's important that the ICMP serialization happens after the payload
            // is written to ensure a correct checksum.
            icmp_repr.serialize(&mut icmp_packet).unwrap();
        })
    }

    fn recv_icmp_packet(&mut self, ipv4_repr: &Ipv4Repr, icmp_buffer: &[u8]) -> Result<()> {
        let icmp_recv_packet = Icmpv4Packet::try_new(icmp_buffer)?;
        icmp_recv_packet.check_encoding()?;

        let icmp_recv_repr = Icmpv4Repr::deserialize(&icmp_recv_packet)?;

        let (ipv4_repr, icmp_repr, icmp_payload) = match icmp_recv_repr {
            Icmpv4Repr::EchoRequest { id, seq } => {
                debug!(
                    "Got a ping from {}; Sending response...",
                    ipv4_repr.src_addr
                );
                let mut ipv4_repr = ipv4_repr.clone();
                swap(&mut ipv4_repr.src_addr, &mut ipv4_repr.dst_addr);
                (
                    ipv4_repr,
                    Icmpv4Repr::EchoReply { id, seq },
                    icmp_recv_packet.payload(),
                )
            }
            _ => return Err(Error::NoOp),
        };

        self.send_icmp_packet(&ipv4_repr, &icmp_repr, |payload| {
            payload.copy_from_slice(icmp_payload);
        })
    }

    fn send_ipv4_packet_raw<F>(
        &mut self,
        dst_addr: Ipv4Address,
        ipv4_packet_len: usize,
        f: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut [u8]),
    {
        let dst_addr = self.ip_route(dst_addr);
        let eth_dst_addr = self.eth_addr_for_ip(dst_addr)?;
        let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(ipv4_packet_len);

        self.send_eth_frame(eth_frame_len, |eth_frame| {
            eth_frame.set_dst_addr(eth_dst_addr);
            eth_frame.set_payload_type(eth_types::IPV4);
            f(eth_frame.payload_mut());
        })
    }

    fn send_ipv4_packet_with_repr<F>(&mut self, ipv4_repr: &Ipv4Repr, f: F) -> Result<()>
    where
        F: FnOnce(&mut [u8]),
    {
        let (dst_addr, ipv4_packet_len) = (ipv4_repr.dst_addr, ipv4_repr.buffer_len());

        self.send_ipv4_packet_raw(dst_addr, ipv4_packet_len, |ipv4_buffer| {
            let mut ipv4_packet = Ipv4Packet::try_new(ipv4_buffer).unwrap();
            // NOTE: It's important to serialize the Ipv4Repr prior to calling payload_mut()
            // to ensure the header length is written and used when finding where the
            // payload is located in the packet!
            ipv4_repr.serialize(&mut ipv4_packet);
            f(ipv4_packet.payload_mut());
        })
    }

    fn recv_ipv4_packet(&mut self, ipv4_buffer: &[u8], sockets: &mut SocketSet) -> Result<()> {
        let ipv4_packet = Ipv4Packet::try_new(ipv4_buffer)?;
        ipv4_packet.check_encoding()?;

        if ipv4_packet.dst_addr() != *self.dev.ipv4_addr() {
            debug!(
                "Ignoring IPv4 packet with destination {}.",
                ipv4_packet.dst_addr()
            );
            return Err(Error::NoOp);
        }

        for socket in sockets.iter_mut() {
            let packet = Packet::Ipv4(ipv4_packet.as_ref());
            match socket.recv_forward(&packet) {
                _ => {}
            }
        }

        let ipv4_repr = Ipv4Repr::deserialize(&ipv4_packet)?;

        match ipv4_packet.protocol() {
            ipv4_protocols::UDP => self.recv_udp_packet(&ipv4_repr, &ipv4_packet, sockets),
            ipv4_protocols::ICMP => self.recv_icmp_packet(&ipv4_repr, ipv4_packet.payload()),
            i => {
                debug!("Ignoring IPv4 packet with type {}.", i);
                Err(Error::NoOp)
            }
        }
    }

    fn send_eth_frame<F>(&mut self, eth_frame_len: usize, f: F) -> Result<()>
    where
        F: FnOnce(&mut EthernetFrame<&mut [u8]>),
    {
        let mut eth_buffer = vec![0; eth_frame_len];
        let mut eth_frame = EthernetFrame::try_new(&mut eth_buffer[..])?;
        eth_frame.set_src_addr(self.dev.ethernet_addr());

        f(&mut eth_frame);

        self.dev.send(eth_frame.as_ref())?;

        Ok(())
    }

    fn recv_eth_frame(&mut self, eth_buffer: &[u8], sockets: &mut SocketSet) -> Result<()> {
        let eth_frame = EthernetFrame::try_new(eth_buffer)?;

        if eth_frame.dst_addr() != self.dev.ethernet_addr() && !eth_frame.dst_addr().is_broadcast()
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
            eth_types::ARP => self.recv_arp_packet(eth_frame.payload()),
            eth_types::IPV4 => self.recv_ipv4_packet(eth_frame.payload(), sockets),
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
                if target_proto_addr != *self.dev.ipv4_addr() {
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
                            source_proto_addr: *self.dev.ipv4_addr(),
                            target_hw_addr: source_hw_addr,
                            target_proto_addr: source_proto_addr,
                        };

                        debug!(
                            "Sending ARP reply to {}/{}.",
                            source_proto_addr, source_hw_addr
                        );

                        let eth_frame_len =
                            EthernetFrame::<&[u8]>::buffer_len(arp_repr.buffer_len());

                        self.send_eth_frame(eth_frame_len, |eth_frame| {
                            eth_frame.set_dst_addr(source_hw_addr);
                            eth_frame.set_payload_type(eth_types::ARP);
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
                    source_proto_addr: *self.dev.ipv4_addr(),
                    target_hw_addr: EthernetAddress::BROADCAST,
                    target_proto_addr: ipv4_addr,
                };

                debug!("Sending ARP request for {}.", ipv4_addr);

                let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(arp_repr.buffer_len());

                self.send_eth_frame(eth_frame_len, |eth_frame| {
                    eth_frame.set_dst_addr(EthernetAddress::BROADCAST);
                    eth_frame.set_payload_type(eth_types::ARP);
                    arp_repr.serialize(eth_frame.payload_mut()).unwrap();
                })?;

                Err(Error::Address)
            }
        }
    }

    fn ip_route(&self, address: Ipv4Address) -> Ipv4Address {
        if self.dev.ipv4_addr().is_member(address) {
            debug!("{} will be routed through link.", address);
            address
        } else {
            debug!("{} will be routed through default gateway.", address);
            self.default_gateway
        }
    }
}
