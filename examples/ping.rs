extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate rand;
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

// Sends an ICMP ping request to a host.
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

    let ping_addr = matches
        .value_of("ADDRESS")
        .map(|addr| Ipv4Address::from_str(addr).unwrap())
        .expect("Bad IP address!");

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(&mut interface, RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    println!("PING {} ({}) 64 bytes of data.", ping_addr, ping_addr);

    for seq in 0 .. 64 {
        let mut payload = [0; 64];

        for i in 0 .. payload.len() {
            payload[i] = rand::random::<u8>();
        }

        match ping(
            &mut interface,
            &mut socket_set,
            raw_handle,
            ping_addr,
            seq,
            0,
            &payload,
            *TIMEOUT,
        ) {
            Some(time) => println!(
                "{} bytes from {}: icmp_seq={} time={:.2} ms",
                payload.len(),
                ping_addr,
                seq,
                (time.as_secs() as f64) * 1000.0 + (time.subsec_nanos() as f64) / 1000000.0,
            ),
            None => println!("Request timeout for icmp_seq {}", seq),
        }

        thread::sleep(Duration::from_secs(1));
    }
}
