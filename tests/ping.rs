extern crate env_logger;
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
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};
use usrnet::examples::*;

fn ping_addr(context: &mut context::Context, addr: Ipv4Address) -> Option<Duration> {
    let raw_socket = context.socket_env.raw_socket(RawType::Ipv4);
    let raw_handle = context
        .socket_set
        .add_socket(TaggedSocket::Raw(raw_socket))
        .unwrap();

    let mut payload = [0; 64];
    for i in 0 .. payload.len() {
        payload[i] = rand::random::<u8>();
    }

    ping(
        &mut context.interface,
        &mut context.socket_set,
        raw_handle,
        addr,
        rand::random::<u16>(),
        0,
        &payload,
        *context::ONE_SEC,
    )
}

#[test]
fn ping_default_gateway() {
    context::run(|context| {
        assert!(ping_addr(context, *env::DEFAULT_IPV4_GATEWAY).unwrap() < *context::ONE_SEC);
    });
}

#[test]
fn ping_google_dns_servers() {
    context::run(|context| {
        assert!(ping_addr(context, Ipv4Address::new([8, 8, 8, 8])).unwrap() < *context::ONE_SEC);
    });
}

#[test]
fn ping_unknown_ip() {
    context::run(|context| {
        assert!(ping_addr(context, *env::NO_HOST_IPV4_ADDR).is_none());
    });
}

#[test]
fn ping_responses() {
    context::run(|context| {
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

        while Instant::now() - start_at < *context::ONE_SEC {
            env::tick(&mut context.interface, &mut context.socket_set);
        }

        ping.join().unwrap();
    });
}
