use usrnet::core::dev::{
    Device,
    Standard,
};
use usrnet::core::link::Link;
use usrnet::core::layers::{
    EthernetAddress,
    Ipv4Address,
};
use usrnet::linux::link::Tap;

#[allow(dead_code)]
pub type Dev = Standard<Tap, &'static mut [u8]>;

static mut DEV_BUFFER: [u8; 10240] = [0; 10240];

pub fn default_dev() -> Dev {
    let tap = Tap::new("tap0");
    let mtu = tap.get_max_transmission_unit().unwrap();
    let dev = Standard::try_new(
        tap,
        unsafe { &mut DEV_BUFFER[..] },
        Ipv4Address::new([10, 0, 0, 103]),
        EthernetAddress::new([0, 1, 2, 3, 4, 5]),
    ).unwrap();

    println!(
        "Device: (MTU = {}, IPv4 = {}, MAC = {})",
        mtu,
        dev.get_ipv4_addr(),
        dev.get_ethernet_addr()
    );

    dev
}
