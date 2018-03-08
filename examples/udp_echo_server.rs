extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod env;

use usrnet::core::services::Interface;
use usrnet::core::socket::{
    Bindings,
    SocketAddr,
    SocketSet,
    TaggedSocket,
};

lazy_static! {
    static ref BIND_ADDR: SocketAddr = SocketAddr {
        addr: *env::DEFAULT_IPV4_ADDR,
        port: 4096,
    };
}

/// Echo's all incoming UDP packets back to the sender.
fn main() {
    env_logger::init();

    let mut interface = env::default_interface();
    let bindings = Bindings::new();
    let addr_binding = bindings.bind_udp(*BIND_ADDR).unwrap();
    let socket = TaggedSocket::Udp(env::udp_socket(addr_binding));

    let mut socket_set = env::socket_set();
    let handle = socket_set.add_socket(socket).unwrap();

    let mut buffer = [0; 4096];

    println!("Running UDP echo server; You can use udp_echo_client.py to generate UDP packets.");

    loop {
        echo(&mut interface, &mut socket_set, handle, &mut buffer[..]);
    }
}

fn echo(interface: &mut Interface, socket_set: &mut SocketSet, handle: usize, buffer: &mut [u8]) {
    let (payload_len, addr) = recv(interface, socket_set, handle, buffer);

    println!("Echo {:?} from {}!", &buffer[.. payload_len], addr);

    socket_set
        .socket(handle)
        .as_udp_socket()
        .send(payload_len, addr)
        .map(|payload| payload.copy_from_slice(&buffer[.. payload_len]))
        .unwrap();
}

fn recv(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    handle: usize,
    buffer: &mut [u8],
) -> (usize, SocketAddr) {
    loop {
        if let Ok((payload, addr)) = socket_set.socket(handle).as_udp_socket().recv() {
            (&mut buffer[.. payload.len()]).copy_from_slice(payload);
            return (payload.len(), addr);
        }

        env::tick(interface, socket_set);
    }
}
