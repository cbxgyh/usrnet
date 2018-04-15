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

/// Opens a TCP communication with an endpoint, sending data from stdin and
/// displays the responses.
fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg ADDRESS: +takes_value +required "IP address to connect to to")
        (@arg PORT:    +takes_value +required "TCP port to connect to on the end host")
    ).get_matches();

    let addr = matches
        .value_of("ADDRESS")
        .and_then(|addr| Ipv4Address::from_str(addr).ok())
        .expect("Bad IP address!");

    let port = matches
        .value_of("PORT")
        .and_then(|port| port.parse::<u16>().ok())
        .expect("Bad TCP port!");

    let server_addr = SocketAddr { addr, port };

    let mut interface = env::default_interface();
    let bindings = Bindings::new();
    let socket_addr = SocketAddr {
        addr: *interface.ipv4_addr,
        port: rand::random::<u16>(),
    };
    let addr_binding = bindings.bind_tcp(socket_addr).unwrap();
    let tcp_socket = TaggedSocket::Tcp(env::tcp_socket(&mut interface, addr_binding));
    let mut socket_set = env::socket_set();
    let tcp_handle = socket_set.add_socket(tcp_socket).unwrap();

    println!(
        "Connecting to {}; \
         Use 'ncat -l -k -p {} -e /bin/cat' to run an echo server.",
        server_addr, server_addr.port
    );

    socket_set
        .socket(tcp_handle)
        .as_tcp_socket()
        .connect(server_addr);
    while socket_set
        .socket(tcp_handle)
        .as_tcp_socket()
        .is_establishing()
    {
        env::tick(&mut interface, &mut socket_set);
    }

    println!("Connection established!");

    if !socket_set.socket(tcp_handle).as_tcp_socket().is_connected() {
        panic!("Error connecting to {}!", server_addr);
    }

    loop {
        env::tick(&mut interface, &mut socket_set);
    }
}
