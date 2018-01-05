use std;

/// [IPv4 address](https://en.wikipedia.org/wiki/IPv4) in network byte order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address([u8; 4]);

impl Address {
    /// Creates an IPv4 address from a network byte order buffer.
    pub fn new(addr: [u8; 4]) -> Address {
        Address(addr)
    }

    /// Returns a reference to the network byte order representation of the address.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}
