#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate usrnet;

use std::str::FromStr;
use std::thread;
use std::time::Duration;

use usrnet::core::repr::Ipv4Address;
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};
use usrnet::examples::*;

/// Sends an ARP request for an IPv4 address.
fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg ADDRESS:    +takes_value +required "Address to arping")
        (@arg TIMEOUT:    +takes_value --timeout "Timeout in milliseconds for each ARP packet")
    ).get_matches();

    let arping_addr = matches
        .value_of("ADDRESS")
        .and_then(|addr| Ipv4Address::from_str(addr).ok())
        .expect("Bad IP address!");

    let timeout = matches
        .value_of("TIMEOUT")
        .or(Some("1000"))
        .and_then(|timeout| timeout.parse::<u64>().ok())
        .map(|timeout| Duration::from_millis(timeout))
        .expect("Bad timeout!");

    let mut interface = env::default_interface();
    let socket_env = env::socket_env(&mut interface);
    let mut socket_set = env::socket_set();

    let raw_socket = socket_env.raw_socket(RawType::Ethernet);
    let raw_handle = socket_set
        .add_socket(TaggedSocket::Raw(raw_socket))
        .unwrap();

    println!("ARPING {}.", arping_addr);

    for i in 0 .. 64 {
        match arping(
            &mut interface,
            &mut socket_set,
            raw_handle,
            arping_addr,
            timeout,
        ) {
            Some((time, eth_addr)) => println!(
                "28 bytes from {} ({}) index={} time={:.2} ms",
                eth_addr,
                arping_addr,
                i,
                (time.as_secs() as f64) * 1000.0 + (time.subsec_nanos() as f64) / 1000000.0,
            ),
            None => println!("Timeout"),
        }

        thread::sleep(Duration::from_secs(1));
    }
}
