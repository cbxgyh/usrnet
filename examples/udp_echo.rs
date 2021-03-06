#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate usrnet;

use usrnet::core::socket::{
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::*;

/// Starts a UDP server that echo's packets to the sender.
fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg PORT: +takes_value +required "UDP port to bind")
    ).get_matches();

    let port = matches
        .value_of("PORT")
        .and_then(|port| port.parse::<u16>().ok())
        .expect("Bad UDP port!");

    let mut interface = env::default_interface();
    let socket_env = env::socket_env(&mut interface);
    let mut socket_set = env::socket_set();

    let socket_addr = SocketAddr {
        addr: *interface.ipv4_addr,
        port,
    };
    let udp_socket = socket_env.udp_socket(socket_addr).unwrap();
    let udp_handle = socket_set
        .add_socket(TaggedSocket::Udp(udp_socket))
        .unwrap();

    println!(
        "Running UDP echo server; Use 'ncat -u {} {}' to send packets.",
        socket_addr.addr, socket_addr.port
    );

    udp_echo(&mut interface, &mut socket_set, udp_handle, || true);
}
