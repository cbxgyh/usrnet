extern crate env_logger;
extern crate usrnet;

mod env;

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let mut _dev = env::default_dev();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
