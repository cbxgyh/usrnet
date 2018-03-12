#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate rand;
extern crate usrnet;

use std::str::FromStr;
use std::time::Duration;

use usrnet::core::repr::Ipv4Address;
use usrnet::core::socket::{
    RawType,
    TaggedSocket,
};
use usrnet::examples::*;

fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg ADDRESS:    +takes_value +required "Address to traceroute")
        (@arg MAX_TTL:    +takes_value --ttl     "Max hops/TTL for each probing packet")
        (@arg TIMEOUT:    +takes_value --timeout "Timeout in milliseconds for each packet")
        (@arg PACKET_LEN: +takes_value --len     "Payload size in bytes for each packet")
    ).get_matches();

    let trace_addr = matches
        .value_of("ADDRESS")
        .and_then(|addr| Ipv4Address::from_str(addr).ok())
        .expect("Bad IP address!");

    let max_ttl = matches
        .value_of("MAX_TTL")
        .or(Some("64"))
        .and_then(|ttl| ttl.parse::<u8>().ok())
        .expect("Bad TTL!");

    let timeout = matches
        .value_of("TIMEOUT")
        .or(Some("1000"))
        .and_then(|timeout| timeout.parse::<u64>().ok())
        .map(|timeout| Duration::from_millis(timeout))
        .expect("Bad timeout!");

    let packet_len = matches
        .value_of("PACKET_LEN")
        .or(Some("64"))
        .and_then(|packet_len| packet_len.parse::<usize>().ok())
        .expect("Bad packet length!");

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();
    let raw_socket = TaggedSocket::Raw(env::raw_socket(&mut interface, RawType::Ipv4));
    let raw_handle = socket_set.add_socket(raw_socket).unwrap();

    println!(
        "traceroute to {} ({}), {} hops max, {} byte packets",
        trace_addr, trace_addr, max_ttl, packet_len
    );

    let reached = traceroute(
        &mut interface,
        &mut socket_set,
        raw_handle,
        trace_addr,
        packet_len,
        max_ttl,
        timeout,
        |ttl, hop| {
            if let Some((time, address)) = hop {
                println!(
                    "{:2} {} ({}) {:.3} ms",
                    ttl,
                    address,
                    address,
                    (time.as_secs() as f64) * 1000.0 + (time.subsec_nanos() as f64) / 1000000.0
                );
            } else {
                println!("{:2} * * * ", ttl);
            }
        },
    ).is_some();

    if !reached {
        std::process::exit(1);
    }
}
