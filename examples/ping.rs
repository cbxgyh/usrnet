extern crate clap;
extern crate usrnet;

mod env;

use usrnet::core::dev::Device;
use usrnet::core::layers::{
    ethernet_types,
    EthernetAddress,
    EthernetFrame,
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    Ipv4Packet,
    ipv4_flags,
    ipv4_types,
};

/// Sends an ICMP echo request to an IPv4 address.
fn main() {
    let mut dev = env::default_dev();
    let ip_addr = Ipv4Address::new([10, 0, 0, 1]);
    send_icmp_packet(&mut dev, ip_addr, |icmp_packet| {
        let icmp = Icmpv4Repr::EchoRequest { id: 42, seq: 1 };
        icmp.serialize(icmp_packet);
    });
    println!(
        "Ping for {} sent. Use tshark or tcpdump to observe.",
        ip_addr
    );
    std::thread::sleep(std::time::Duration::from_secs(1));
}

fn send_icmp_packet<F>(dev: &mut env::Dev, ip_addr: Ipv4Address, f: F)
where
    F: FnOnce(&mut Icmpv4Packet<&mut [u8]>),
{
    send_ipv4_packet(dev, Icmpv4Packet::<&[u8]>::MIN_BUFFER_LEN, |ip_packet| {
        ip_packet.set_protocol(ipv4_types::ICMP as u8);
        ip_packet.set_dst_addr(ip_addr);

        let mut icmp_packet = Icmpv4Packet::try_from(ip_packet.payload_mut()).unwrap();
        icmp_packet.set_checksum(0);

        f(&mut icmp_packet);

        let checksum = icmp_packet.gen_checksum();
        icmp_packet.set_checksum(checksum);
    });
}

fn send_ipv4_packet<F>(dev: &mut env::Dev, buffer_len: usize, f: F)
where
    F: FnOnce(&mut Ipv4Packet<&mut [u8]>),
{
    let src_ip_addr = dev.get_ipv4_addr();
    let packet_len = Ipv4Packet::<&[u8]>::buffer_len(buffer_len);
    send_eth_frame(dev, packet_len, |eth_frame| {
        // TODO: Arp cache.
        eth_frame.set_dst_addr(EthernetAddress::new([0x0A, 0x00, 0x27, 0x00, 0x00, 0x00]));
        eth_frame.set_payload_type(ethernet_types::IPV4 as u16);

        let mut ip_packet = Ipv4Packet::try_from(eth_frame.payload_mut()).unwrap();
        ip_packet.set_ip_version(4);
        ip_packet.set_header_len(5);
        ip_packet.set_dscp(0);
        ip_packet.set_ecn(0);
        ip_packet.set_packet_len(packet_len as u16);
        ip_packet.set_identification(42);
        ip_packet.set_flags(ipv4_flags::DONT_FRAGMENT);
        ip_packet.set_fragment_offset(0);
        ip_packet.set_ttl(64);
        ip_packet.set_header_checksum(0);
        ip_packet.set_src_addr(src_ip_addr);

        f(&mut ip_packet);

        let header_checksum = ip_packet.gen_header_checksum();
        ip_packet.set_header_checksum(header_checksum);
    });
}

fn send_eth_frame<F>(dev: &mut env::Dev, buffer_len: usize, f: F)
where
    F: FnOnce(&mut EthernetFrame<&mut [u8]>),
{
    let src_hw_addr = dev.get_ethernet_addr();
    let frame_len = EthernetFrame::<&[u8]>::buffer_len(buffer_len);
    let mut buffer = dev.send(frame_len).unwrap();
    let mut ethernet_frame = EthernetFrame::try_from(buffer.as_mut()).unwrap();
    ethernet_frame.set_src_addr(src_hw_addr);
    f(&mut ethernet_frame);
}
