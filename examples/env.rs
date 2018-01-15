use usrnet::core::dev::{
    Device,
    Standard,
};
use usrnet::core::link::Link;
use usrnet::core::repr::{
    Ipv4,
    Mac,
};
use usrnet::linux::link::Tap;

pub fn default_dev() -> Standard<Tap> {
    let tap = Tap::new("tap0");
    let mtu = tap.get_max_transmission_unit().unwrap();
    let dev = Standard::new(
        tap,
        Ipv4::new([10, 0, 0, 103]),
        Mac::new([0, 1, 2, 3, 4, 5]),
    ).unwrap();

    println!(
        "Device: (MTU = {}, IPv4 = {}, MAC = {})",
        mtu,
        dev.get_ipv4_addr(),
        dev.get_ethernet_addr()
    );

    dev
}
