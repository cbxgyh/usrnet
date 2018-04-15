#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate usrnet;

mod context;

use std::net::{
    SocketAddr as StdSocketAddr,
    UdpSocket,
};
use std::sync::mpsc;
use std::thread;

use usrnet::core::socket::{
    Bindings,
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::{
    env,
    udp_echo as _udp_echo,
};

pub const PAYLOAD_SIZE: usize = 128;

pub const CONCURRENT_CLIENTS: usize = 10;

pub const PACKETS_PER_CLIENT: usize = 1000;

fn std_udp_echo_client(server_addr: StdSocketAddr, sender: mpsc::Sender<()>) {
    let socket = UdpSocket::bind("0:0").unwrap();

    for _ in 0 .. PACKETS_PER_CLIENT {
        let mut recv = [0; PAYLOAD_SIZE + 1];
        let mut send = [0; PAYLOAD_SIZE];
        for i in 0 .. PAYLOAD_SIZE {
            send[i] = rand::random::<u8>();
        }

        assert_eq!(socket.send_to(&send, server_addr).unwrap(), PAYLOAD_SIZE);

        loop {
            let (size, socket_addr) = socket.recv_from(&mut recv).unwrap();
            if socket_addr == server_addr {
                assert_eq!(size, PAYLOAD_SIZE);
                assert_eq!(&recv[.. PAYLOAD_SIZE], &send[..]);
                break;
            }
        }
    }

    sender.send(()).unwrap();
}

#[test]
fn udp_echo() {
    context::run(|context| {
        let server_addr = SocketAddr {
            addr: *context.interface.ipv4_addr,
            port: context::rand_port(),
        };

        let (send, recv) = mpsc::channel();

        for _ in 0 .. CONCURRENT_CLIENTS {
            let send_clone = send.clone();
            thread::spawn(move || {
                std_udp_echo_client(StdSocketAddr::V4(server_addr.into()), send_clone)
            });
        }

        let bindings = Bindings::new();
        let addr_binding = bindings.bind_udp(server_addr).unwrap();
        let udp_socket = TaggedSocket::Udp(env::udp_socket(&mut context.interface, addr_binding));
        let mut socket_set = env::socket_set();
        let udp_handle = socket_set.add_socket(udp_socket).unwrap();

        let mut waiting = CONCURRENT_CLIENTS;
        _udp_echo(&mut context.interface, &mut socket_set, udp_handle, || {
            while let Ok(_) = recv.try_recv() {
                waiting -= 1;
            }
            waiting > 0
        });
    });
}
