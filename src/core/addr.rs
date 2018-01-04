/// [MAC address](https://en.wikipedia.org/wiki/MAC_address) in network byte
/// order.
pub type Mac = [u8; 6];

pub const ETH_BROADCAST: Mac = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff];

/// [IPv4 address](https://en.wikipedia.org/wiki/IPv4) in network byte order.
pub type Ipv4 = [u8; 4];
