#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod context;

use std::net::{
    SocketAddr as StdSocketAddr,
    UdpSocket,
};
use std::thread;
use std::time::Duration;

use usrnet::core::socket::{
    Bindings,
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::*;

lazy_static! {
    static ref TIMEOUT: Duration = Duration::from_secs(1);

    static ref ECHO_ADDR: StdSocketAddr = "10.0.0.102:4096"
        .parse()
        .unwrap();
}

pub static NUM_CLIENTS: usize = 10;

pub static NUM_PACKETS: usize = 1000;

pub static PORT: u16 = 4096;

#[test]
fn echo_udp_packet() {
    context::run(|interface, _| {
        let mut workers = vec![];

        for _ in 0 .. NUM_CLIENTS {
            workers.push(thread::spawn(|| {
                let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

                for i in 0 .. NUM_PACKETS {
                    let (mut rx, mut tx) = ([0; 128], [0; 128]);
                    for k in 0 .. tx.len() {
                        tx[k] = (i * 10 + k) as u8;
                    }

                    assert_eq!(128, socket.send_to(&tx, *ECHO_ADDR).unwrap());

                    loop {
                        let (len, src) = socket.recv_from(&mut rx).unwrap();
                        if src != *ECHO_ADDR {
                            continue;
                        }
                        assert_eq!(src, *ECHO_ADDR);
                        assert_eq!(len, tx.len());
                        assert_eq!(&tx[..], &rx[..]);
                        break;
                    }
                }
            }));
        }

        let bindings = Bindings::new();
        let sock_addr = SocketAddr {
            addr: *interface.ipv4_addr,
            port: PORT,
        };
        let addr_binding = bindings.bind_udp(sock_addr).unwrap();
        let udp_socket = TaggedSocket::Udp(env::udp_socket(interface, addr_binding));
        let mut socket_set = env::socket_set();
        let udp_handle = socket_set.add_socket(udp_socket).unwrap();

        for _ in 0 .. (NUM_CLIENTS * NUM_PACKETS) {
            udp_echo(interface, &mut socket_set, udp_handle, *TIMEOUT).unwrap();
        }

        for worker in workers {
            worker.join().unwrap();
        }
    });
}
