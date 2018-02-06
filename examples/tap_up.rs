extern crate clap;
extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::core::layers::{
    Icmpv4Packet,
    Icmpv4Repr,
    Ipv4Address,
    ipv4_types,
};

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let mut service = env::default_service();

    let buffer_len = Icmpv4Packet::<&[u8]>::MIN_BUFFER_LEN;
    let ip_addr = Ipv4Address::new([10, 0, 0, 1]);

    // Loop until ARP resolves IP address...
    loop {
        service.recv();

        match service.send_ipv4_packet(buffer_len, ip_addr, |ipv4_packet| {
            ipv4_packet.set_protocol(ipv4_types::ICMP);
            ipv4_packet.set_dst_addr(ip_addr);

            let mut icmp_packet = Icmpv4Packet::try_from(ipv4_packet.payload_mut()).unwrap();
            icmp_packet.set_checksum(0);

            let icmp_repr = Icmpv4Repr::EchoRequest { id: 42, seq: 1 };
            icmp_repr.serialize(&mut icmp_packet);

            let checksum = icmp_packet.gen_checksum();
            icmp_packet.set_checksum(checksum);
        }) {
            Ok(_) => break,
            _ => {}
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Sent a ping to {} via Tap!", ip_addr);
}
