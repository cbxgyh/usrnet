#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate usrnet;

use usrnet::core::socket::{
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::*;

/// Starts a TCP server that echo's an incoming stream to the sender.
fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg PORT: +takes_value +required "TCP port to bind")
    ).get_matches();

    let port = matches
        .value_of("PORT")
        .and_then(|port| port.parse::<u16>().ok())
        .expect("Bad TCP port!");

    let mut interface = env::default_interface();
    let socket_env = env::socket_env(&mut interface);
    let mut socket_set = env::socket_set();

    let socket_addr = SocketAddr {
        addr: *interface.ipv4_addr,
        port,
    };
    let tcp_socket = socket_env.tcp_socket(socket_addr).unwrap();
    let tcp_handle = socket_set
        .add_socket(TaggedSocket::Tcp(tcp_socket))
        .unwrap();

    println!(
        "Running TCP echo server; Use 'ncat {} {}' to send messages.",
        socket_addr.addr, socket_addr.port
    );

    loop {
        tcp_echo(&mut interface, &mut socket_set, tcp_handle, || true);
    }
}
