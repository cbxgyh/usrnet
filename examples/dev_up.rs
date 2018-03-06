extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate usrnet;

mod env;

/// Opens and brings UP a Linux TAP interface. You should be able to issue ping
/// requests to env::DEFAULT_IPV4_ADDR and get responses.
fn main() {
    env_logger::init();

    let mut service = env::default_service();
    let mut socket_set = env::socket_set();

    loop {
        env::tick(&mut service, &mut socket_set);
    }
}
