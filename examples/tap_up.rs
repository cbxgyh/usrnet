extern crate clap;
extern crate usrnet;

mod cli;

use cli::App;
use usrnet::core::link::Link;
use usrnet::linux::link::Tap;

/// Opens and brings UP a Linux TAP interface.
fn main() {
    let matches = clap::App::new("tap_up")
        .about("Opens a Linux TAP interface to bring it into the UP state")
        .with_defaults()
        .get_matches();
    let interface = matches.value_of("tap").unwrap();
    let tap = Tap::new(interface);

    println!(
        "{} MTU: {}",
        interface,
        tap.get_max_transmission_unit().unwrap()
    );
    println!("{} is UP!", interface);

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
