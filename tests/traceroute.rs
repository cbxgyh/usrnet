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
use usrnet::core::service::Interface;
use usrnet::core::socket::{
    RawType,
    SocketSet,
    TaggedSocket,
};
use usrnet::examples::*;

lazy_static! {
    static ref TIMEOUT: Duration = Duration::from_millis(50);

    static ref MAX_TTL: u8 = 16;
}

fn traceroute_addr<F>(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    addr: Ipv4Address,
    f: F,
) -> Option<()>
where
    F: FnMut(u8, Option<(Duration, Ipv4Address)>),
{
    let raw_socket = TaggedSocket::Raw(env::raw_socket(interface, RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    traceroute(
        interface,
        socket_set,
        raw_handle,
        addr,
        64,
        *MAX_TTL,
        *TIMEOUT,
        f,
    )
}

#[test]
fn traceroute_default_gateway() {
    context::run(|interface, socket_set| {
        let (mut callbacks, mut addr) = (0, Ipv4Address::new([0, 0, 0, 0]));

        assert!(
            traceroute_addr(
                interface,
                socket_set,
                *env::DEFAULT_IPV4_GATEWAY,
                |ttl, hop| {
                    assert_eq!(ttl, 1);
                    assert_eq!(callbacks, 0);
                    callbacks += 1;
                    if let Some((time, response_addr)) = hop {
                        assert!(time < *TIMEOUT);
                        addr = response_addr;
                    }
                }
            ).is_some()
        );

        assert_eq!(callbacks, 1);
        assert_eq!(addr, *env::DEFAULT_IPV4_GATEWAY);
    });
}

#[test]
fn traceroute_unknown_ip() {
    context::run(|interface, socket_set| {
        let mut callbacks = 0;

        assert!(
            traceroute_addr(
                interface,
                socket_set,
                Ipv4Address::new([10, 0, 0, 128]),
                |ttl, _| {
                    assert!(ttl <= *MAX_TTL);
                    callbacks += 1;
                }
            ).is_none()
        );

        assert_eq!(callbacks, *MAX_TTL);
    });
}

#[test]
fn traceroute_responses() {
    context::run(|interface, socket_set| {
        let traceroute = thread::spawn(|| {
            let output = context::Output::from(
                Command::new("traceroute")
                    .args(&["-m", "1", "-w", "1", "10.0.0.102"])
                    .output()
                    .unwrap(),
            );
            assert!(output.status.success());
        });

        let start_at = Instant::now();

        while Instant::now() - start_at < Duration::from_secs(1) {
            env::tick(interface, socket_set);
        }

        traceroute.join().unwrap();
    });
}
