#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate rand;
extern crate usrnet;

use std::io;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

use usrnet::core::repr::Ipv4Address;
use usrnet::core::socket::{
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::*;

/// Sends UDP packets to an endpoint, sending data from stdin and displays the
/// responses.
fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg ADDRESS: +takes_value +required "IP address to send UDP packets to")
        (@arg PORT:    +takes_value +required "UDP port to send packets to on the end host")
    ).get_matches();

    let addr = matches
        .value_of("ADDRESS")
        .and_then(|addr| Ipv4Address::from_str(addr).ok())
        .expect("Bad IP address!");

    let port = matches
        .value_of("PORT")
        .and_then(|port| port.parse::<u16>().ok())
        .expect("Bad UDP port!");

    let server_addr = SocketAddr { addr, port };

    let mut interface = env::default_interface();
    let socket_env = env::socket_env(&mut interface);
    let mut socket_set = env::socket_set();

    let socket_addr = SocketAddr {
        addr: *interface.ipv4_addr,
        port: rand::random::<u16>(),
    };
    let udp_socket = socket_env.udp_socket(socket_addr).unwrap();
    let udp_handle = socket_set
        .add_socket(TaggedSocket::Udp(udp_socket))
        .unwrap();

    let (send, recv) = mpsc::channel();

    thread::spawn(move || loop {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        send.send(buf).unwrap();
    });

    println!(
        "Sending and receiving UDP packets to/from {}; \
         Use 'ncat -l -u -k -p {} -e /bin/cat' to run an echo server.",
        server_addr, server_addr.port
    );

    loop {
        if let Ok(buf) = recv.try_recv() {
            socket_set
                .socket(udp_handle)
                .as_udp_socket()
                .send(buf.as_bytes().len(), server_addr)
                .unwrap()
                .copy_from_slice(buf.as_bytes());
        }

        if let Ok((buf, _)) = socket_set.socket(udp_handle).as_udp_socket().recv() {
            println!("{}", String::from_utf8_lossy(buf));
        }

        env::tick(&mut interface, &mut socket_set);
    }
}
