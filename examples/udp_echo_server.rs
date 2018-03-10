extern crate clap;
extern crate env_logger;
extern crate usrnet;

use std::time::Duration;

use clap::{
    App,
    Arg,
};

use usrnet::core::socket::{
    Bindings,
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::*;

/// Echo's incoming UDP packets to the sender.
fn main() {
    env_logger::init();

    let matches = App::new("ping")
        .arg(Arg::with_name("PORT").value_name("PORT").takes_value(true))
        .get_matches();

    let port = matches
        .value_of("PORT")
        .or(Some("4096"))
        .and_then(|port| port.parse::<u16>().ok())
        .expect("Bad UDP port!");

    let mut interface = env::default_interface();
    let bindings = Bindings::new();
    let sock_addr = SocketAddr {
        addr: *interface.ipv4_addr,
        port,
    };
    let addr_binding = bindings.bind_udp(sock_addr).unwrap();
    let udp_socket = TaggedSocket::Udp(env::udp_socket(&mut interface, addr_binding));

    let mut socket_set = env::socket_set();
    let udp_handle = socket_set.add_socket(udp_socket).unwrap();

    println!("Running UDP echo server; You can use udp_echo_client.py to generate UDP packets.");

    loop {
        udp_echo(
            &mut interface,
            &mut socket_set,
            udp_handle,
            Duration::from_secs(60),
        );
    }
}
