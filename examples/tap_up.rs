extern crate env_logger;
extern crate usrnet;

mod env;

use std::thread;
use std::time::Duration;

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let mut _dev = env::default_dev();

    loop {
        thread::sleep(Duration::from_millis(1000));
    }
}
