#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate usrnet;

mod context;

use std::net::{
    SocketAddr as StdSocketAddr,
    UdpSocket,
};
use std::thread;

use usrnet::core::socket::{
    Bindings,
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::*;

pub const PAYLOAD_SIZE: usize = 128;

pub const NUM_CLIENTS: usize = 10;

pub const NUM_PACKETS: usize = 1000;

fn udp_echo_client(server_addr: StdSocketAddr) {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

    for _ in 0 .. NUM_PACKETS {
        let (mut recv, mut send) = ([0; PAYLOAD_SIZE], [0; PAYLOAD_SIZE]);
        for i in 0 .. PAYLOAD_SIZE {
            send[i] = rand::random::<u8>();
        }

        assert_eq!(128, socket.send_to(&send, server_addr).unwrap());

        loop {
            let (_, socket_addr) = socket.recv_from(&mut recv).unwrap();
            if socket_addr == server_addr {
                assert_eq!(&send[..], &recv[..]);
                break;
            }
        }
    }
}

#[test]
fn udp_echo_server() {
    context::run(|context| {
        let server_addr = SocketAddr {
            addr: *context.interface.ipv4_addr,
            port: context::rand_port(),
        };

        let workers: Vec<_> = (0 .. NUM_CLIENTS)
            .map(|_| thread::spawn(move || udp_echo_client(StdSocketAddr::V4(server_addr.into()))))
            .collect();

        let bindings = Bindings::new();
        let addr_binding = bindings.bind_udp(server_addr).unwrap();
        let udp_socket = TaggedSocket::Udp(env::udp_socket(&mut context.interface, addr_binding));
        let mut socket_set = env::socket_set();
        let udp_handle = socket_set.add_socket(udp_socket).unwrap();

        for _ in 0 .. (NUM_CLIENTS * NUM_PACKETS) {
            udp_echo(
                &mut context.interface,
                &mut socket_set,
                udp_handle,
                *context::ONE_SEC,
            ).unwrap();
        }

        for worker in workers {
            worker.join().unwrap();
        }
    });
}
