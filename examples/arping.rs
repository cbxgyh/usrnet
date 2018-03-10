extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

use std::str::FromStr;
use std::thread;
use std::time::Duration;

use clap::{
    App,
    Arg,
};

use usrnet::core::repr::Ipv4Address;
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};
use usrnet::examples::*;

lazy_static! {
    static ref TIMEOUT: Duration = Duration::from_millis(1000);
}

/// Sends an ARP request for an IPv4 address.
fn main() {
    env_logger::init();

    let matches = App::new("ping")
        .arg(
            Arg::with_name("ADDRESS")
                .value_name("ADDRESS")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let arping_addr = matches
        .value_of("ADDRESS")
        .map(|addr| Ipv4Address::from_str(addr).unwrap())
        .expect("Bad IP address!");

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(&mut interface, RawType::Ethernet));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    println!("ARPING {}.", arping_addr);

    for i in 0 .. 64 {
        match arping(
            &mut interface,
            &mut socket_set,
            raw_handle,
            arping_addr,
            *TIMEOUT,
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
