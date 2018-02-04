extern crate clap;
extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::core::service::Service;

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let dev = env::default_dev();
    let mut service = Service::new(dev);

    println!("tap0 is UP!");

    loop {
        service.recv();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
