extern crate env_logger;
extern crate usrnet;

use usrnet::examples::*;

/// Opens and brings UP a Linux TAP interface. You should be able to issue ping
/// requests to env::DEFAULT_IPV4_ADDR and get responses.
fn main() {
    env_logger::init();

    let mut interface = env::default_interface();
    let mut socket_set = env::socket_set();

    loop {
        env::tick(&mut interface, &mut socket_set);
    }
}
