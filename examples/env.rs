use usrnet::core::dev::Device;
use usrnet::core::layers::{
    EthernetAddress,
    Ipv4Address,
};
use usrnet::linux::dev::Tap;

pub type Dev = Tap;

static mut DEV_BUFFER: [u8; 10240] = [0; 10240];

pub fn default_dev() -> Dev {
    let tap = Tap::new(
        "tap0",
        Ipv4Address::new([10, 0, 0, 103]),
        EthernetAddress::new([0, 1, 2, 3, 4, 5]),
    );

    println!(
        "Device: (MTU = {}, IPv4 = {}, MAC = {})",
        tap.max_transmission_unit(),
        tap.ipv4_addr(),
        tap.ethernet_addr()
    );

    tap
}

#[allow(dead_code)]
pub fn mut_buffer(buffer_len: usize) -> &'static mut [u8] {
    unsafe {
        let buffer = &mut DEV_BUFFER[..buffer_len];
        for i in 0..buffer_len {
            buffer[i] = 0
        }
        buffer
    }
}
