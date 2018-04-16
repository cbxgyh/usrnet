#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate rand;
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

// Sends an ICMP ping request to a host.
fn main() {
    env_logger::init();

    let matches = clap_app!(app =>
        (@arg ADDRESS:    +takes_value +required "Address to ping")
        (@arg TIMEOUT:    +takes_value --timeout "Timeout in milliseconds for each ICMP packet")
        (@arg PACKET_LEN: +takes_value --len     "Payload size in bytes for each ICMP packet")
    ).get_matches();

    let ping_addr = matches
        .value_of("ADDRESS")
        .and_then(|addr| Ipv4Address::from_str(addr).ok())
        .expect("Bad IP address!");

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
    let socket_env = env::socket_env(&mut interface);
    let mut socket_set = env::socket_set();

    let raw_socket = socket_env.raw_socket(RawType::Ipv4);
    let raw_handle = socket_set
        .add_socket(TaggedSocket::Raw(raw_socket))
        .unwrap();

    println!(
        "PING {} ({}) {} bytes of data.",
        ping_addr, ping_addr, packet_len
    );

    for seq in 0 .. 64 {
        let mut payload = vec![0; packet_len];

        for i in 0 .. packet_len {
            payload[i] = rand::random::<u8>();
        }

        match ping(
            &mut interface,
            &mut socket_set,
            raw_handle,
            ping_addr,
            seq,
            rand::random::<u16>(),
            &payload,
            timeout,
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
