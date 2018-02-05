extern crate clap;
extern crate env_logger;
extern crate usrnet;

mod env;

use usrnet::core::arp_cache::ArpCache;
use usrnet::core::service::Service;
use usrnet::core::time::SystemEnv;

/// Opens and brings UP a Linux TAP interface.
fn main() {
    env_logger::init();

    let dev = env::default_dev();
    let arp_cache = ArpCache::new(60, SystemEnv::new());
    let mut service = Service::new(dev, arp_cache);

    println!("tap0 is UP!");

    loop {
        service.recv();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
