#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate rand;
extern crate usrnet;

use std::str::FromStr;

use usrnet::core::repr::Ipv4Address;
use usrnet::core::socket::{
    Bindings,
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::*;

/// Opens a TCP connection, expecting to receive an equivalent stream in
/// response.
fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg ADDRESS: +takes_value "IP address of the echo server")
        (@arg PORT: +takes_value "TCP port the echo server is running on")
    ).get_matches();

    let echo_addr = matches
        .value_of("ADDRESS")
        .and_then(|addr| Ipv4Address::from_str(addr).ok())
        .expect("Bad IP address!");

    let echo_port = matches
        .value_of("PORT")
        .and_then(|port| port.parse::<u16>().ok())
        .expect("Bad TCP port!");

    let mut interface = env::default_interface();
    let bindings = Bindings::new();
    let sock_addr = SocketAddr {
        addr: *interface.ipv4_addr,
        port: rand::random::<u16>(),
    };
    let addr_binding = bindings.bind_udp(sock_addr).unwrap();
    let tcp_socket = TaggedSocket::Tcp(env::tcp_socket(&mut interface, addr_binding));

    let mut socket_set = env::socket_set();
    let tcp_handle = socket_set.add_socket(tcp_socket).unwrap();

    let connect_addr = SocketAddr {
        addr: echo_addr,
        port: echo_port,
    };
    socket_set
        .socket(tcp_handle)
        .as_tcp_socket()
        .connect(connect_addr);

    loop {
        env::tick(&mut interface, &mut socket_set);
    }
}
