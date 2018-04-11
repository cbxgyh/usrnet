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

pub const MAX_TTL: u8 = 10;

fn traceroute_addr<F>(context: &mut context::Context, addr: Ipv4Address, f: F) -> Option<()>
where
    F: FnMut(u8, Option<(Duration, Ipv4Address)>),
{
    let raw_socket = TaggedSocket::Raw(env::raw_socket(&mut context.interface, RawType::Ipv4));
    let raw_handle = context.socket_set.add_socket(raw_socket).unwrap();

    traceroute(
        &mut context.interface,
        &mut context.socket_set,
        raw_handle,
        addr,
        64,
        MAX_TTL,
        *context::ONE_SEC,
        f,
    )
}

#[test]
fn traceroute_default_gateway() {
    context::run(|context| {
        let (mut callbacks, mut addr) = (0, Ipv4Address::new([0, 0, 0, 0]));

        assert!(
            traceroute_addr(context, *env::DEFAULT_IPV4_GATEWAY, |ttl, hop| {
                assert_eq!(ttl, 1);
                assert_eq!(callbacks, 0);
                callbacks += 1;
                if let Some((time, response_addr)) = hop {
                    assert!(time < *context::ONE_SEC);
                    addr = response_addr;
                }
            }).is_some()
        );

        assert_eq!(callbacks, 1);
        assert_eq!(addr, *env::DEFAULT_IPV4_GATEWAY);
    });
}

#[test]
fn traceroute_unknown_ip() {
    context::run(|context| {
        let mut callbacks = 0;

        assert!(
            traceroute_addr(context, Ipv4Address::new([10, 0, 0, 128]), |ttl, _| {
                assert!(ttl <= MAX_TTL);
                callbacks += 1;
            }).is_none()
        );

        assert_eq!(callbacks, MAX_TTL);
    });
}

#[test]
fn traceroute_responses() {
    context::run(|context| {
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

        while Instant::now() - start_at < *context::ONE_SEC {
            env::tick(&mut context.interface, &mut context.socket_set);
        }

        traceroute.join().unwrap();
    });
}
