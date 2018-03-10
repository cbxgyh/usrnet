#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate usrnet;

mod context;

use std::process::Command;
use std::thread;
use std::time::{
    Duration,
    Instant,
};

use usrnet::core::repr::Ipv4Address;
use usrnet::core::service::{
    socket,
    Interface,
};
use usrnet::core::socket::{
    RawType,
    SocketSet,
    TaggedSocket,
};
use usrnet::examples::*;

lazy_static! {
    static ref TIMEOUT: Duration = Duration::from_secs(1);
}

fn ping_addr(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    addr: Ipv4Address,
) -> Option<Duration> {
    let raw_socket = TaggedSocket::Raw(env::raw_socket(interface, RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    let mut payload = [0; 64];
    for i in 0 .. payload.len() {
        payload[i] = rand::random::<u8>();
    }

    ping(
        interface,
        socket_set,
        raw_handle,
        addr,
        rand::random::<u16>(),
        0,
        &payload,
        *TIMEOUT,
    )
}

#[test]
fn ping_default_gateway() {
    context::run(|interface, socket_set| {
        assert!(ping_addr(interface, socket_set, *env::DEFAULT_IPV4_GATEWAY).unwrap() < *TIMEOUT);
    });
}

#[test]
fn ping_unknown_ip() {
    context::run(|interface, socket_set| {
        assert!(ping_addr(interface, socket_set, Ipv4Address::new([10, 0, 0, 128])).is_none());
    });
}

#[test]
fn ping_responses() {
    context::run(|interface, socket_set| {
        let ping = thread::spawn(|| {
            let output = context::Output::from(
                Command::new("ping")
                    .args(&["-c", "1", "-w", "1", "10.0.0.102"])
                    .output()
                    .unwrap(),
            );
            assert!(output.status.success());
        });

        let start_at = Instant::now();

        while Instant::now() - start_at < Duration::from_secs(2) {
            socket::recv(interface, socket_set);
        }

        ping.join().unwrap();
    });
}
