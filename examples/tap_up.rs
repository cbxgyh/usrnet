extern crate clap;
extern crate usrnet;

mod env;

/// Opens and brings UP a Linux TAP interface.
fn main() {
    let _dev = env::default_dev();
    println!("tap0 is UP!");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
