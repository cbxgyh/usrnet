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

impl std::str::FromStr for Address {
    type Err = ();

    /// Parses an Ipv4 address from an A.B.C.D style string.
    fn from_str(addr: &str) -> Result<Address, Self::Err> {
        let (bytes, unknown): (Vec<_>, Vec<_>) = addr.split(".")
            .map(|token| token.parse::<u8>())
            .partition(|byte| !byte.is_err());

        if bytes.len() != 4 || unknown.len() > 0 {
            return Err(());
        }

        let bytes: Vec<_> = bytes.into_iter().map(|byte| byte.unwrap()).collect();

        let mut ipv4: [u8; 4] = [0; 4];
        ipv4.clone_from_slice(&bytes);

        Ok(Address::new(ipv4))
    }
}
