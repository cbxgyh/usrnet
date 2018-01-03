extern crate usrnet;

const MS_HOUR: u64 = 60 * 60 * 24;

fn main() {
    let _link = usrnet::linux::link::Tap::new("tap0");
    std::thread::sleep(std::time::Duration::from_millis(MS_HOUR));
}
