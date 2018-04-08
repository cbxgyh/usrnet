extern crate env_logger;
extern crate rand;
extern crate usrnet;

use usrnet::core::repr::{
    Ipv4Address,
    Ipv4Protocol,
    Ipv4Repr,
    TcpRepr,
};
use usrnet::core::service::tcp;
use usrnet::examples::*;

/// Opens a TCP connection, expecting to receive an equivalent stream in
/// response.
fn main() {
    env_logger::init();

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();

    let mut tcp_repr = TcpRepr {
        src_port: rand::random::<u16>(),
        dst_port: 1024,
        seq_num: 0,
        ack_num: 0,
        flags: [false; 9],
        window_size: 64,
        urgent_pointer: 0,
        max_segment_size: Some(536),
    };

    tcp_repr.flags[TcpRepr::FLAG_SYN] = true;

    let ipv4_repr = Ipv4Repr {
        src_addr: *interface.ipv4_addr,
        dst_addr: Ipv4Address::new([8, 8, 8, 8]),
        protocol: Ipv4Protocol::TCP,
        payload_len: tcp_repr.header_len() as u16,
    };

    while tcp::send_packet(&mut interface, &ipv4_repr, &tcp_repr, |_| {}).is_err() {
        env::tick(&mut interface, &mut socket_set);
    }
}
