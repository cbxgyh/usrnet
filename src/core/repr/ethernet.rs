use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult,
};
use std::io::Write;
use std::result::Result as StdResult;
use std::str::FromStr;

use byteorder::{
    NetworkEndian,
    ReadBytesExt,
    WriteBytesExt,
};

use {
    Error,
    Result,
};

/// [MAC address](https://en.wikipedia.org/wiki/MAC_address) in network byte order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Address([u8; 6]);

impl Address {
    pub const BROADCAST: Address = Address([0xFF; 6]);

    /// Creates a MAC address from a network byte order buffer.
    pub fn new(addr: [u8; 6]) -> Address {
        Address(addr)
    }

    /// Tries to creates a MAC address from a network byte order slice.
    pub fn try_new(addr: &[u8]) -> Result<Address> {
        if addr.len() != 6 {
            return Err(Error::Exhausted);
        }

        let mut _addr: [u8; 6] = [0; 6];
        _addr.clone_from_slice(addr);
        Ok(Address(_addr))
    }

    /// Returns a reference to the network byte order representation of the
    /// address.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    // Checks if this is a unicast address.
    pub fn is_unicast(&self) -> bool {
        !(self.is_multicast() || self.is_broadcast())
    }

    // Checks if this is a multicast address.
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0b00000001) > 0
    }

    /// Checks if this is a broadcast address.
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF; 6]
    }

    /// Checks if this is a locally assigned address or OUI assigned by IEEE.
    pub fn is_local(&self) -> bool {
        (self.0[0] & 0b00000010) > 0
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5],
        )
    }
}

impl FromStr for Address {
    type Err = ();

    /// Parses a MAC address from an A:B:C:D:E:F style string.
    fn from_str(addr: &str) -> StdResult<Address, Self::Err> {
        let (bytes, unknown): (Vec<_>, Vec<_>) = addr.split(":")
            .map(|token| u8::from_str_radix(token, 16))
            .partition(|byte| !byte.is_err());

        if bytes.len() != 6 || unknown.len() > 0 {
            return Err(());
        }

        let bytes: Vec<_> = bytes.into_iter().map(|byte| byte.unwrap()).collect();

        let mut mac: [u8; 6] = [0; 6];
        mac.clone_from_slice(&bytes);

        Ok(Address::new(mac))
    }
}

/// [https://en.wikipedia.org/wiki/EtherType](https://en.wikipedia.org/wiki/EtherType)
pub mod eth_types {
    pub const IPV4: u16 = 0x800;

    pub const ARP: u16 = 0x806;
}

mod fields {
    use std::ops::{
        Range,
        RangeFrom,
    };

    pub const DST_ADDR: Range<usize> = 0 .. 6;

    pub const SRC_ADDR: Range<usize> = 6 .. 12;

    pub const PAYLOAD_TYPE: Range<usize> = 12 .. 14;

    pub const PAYLOAD: RangeFrom<usize> = 14 ..;
}

/// View of a byte buffer as an Ethernet frame.
#[derive(Debug)]
pub struct Frame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> AsRef<[u8]> for Frame<T> {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> AsMut<[u8]> for Frame<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.buffer.as_mut()
    }
}

impl<T: AsRef<[u8]>> Frame<T> {
    pub const HEADER_LEN: usize = 14;

    pub const MAX_FRAME_LEN: usize = 1518;

    /// Tries to create an Ethernet frame view over a byte buffer.
    pub fn try_new(buffer: T) -> Result<Frame<T>> {
        if buffer.as_ref().len() < Self::HEADER_LEN || buffer.as_ref().len() > Self::MAX_FRAME_LEN {
            Err(Error::Exhausted)
        } else {
            Ok(Frame { buffer })
        }
    }

    /// Returns the length of an Ethernet frame with the specified payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        Self::HEADER_LEN + payload_len
    }

    pub fn dst_addr(&self) -> Address {
        Address::try_new(&self.buffer.as_ref()[fields::DST_ADDR]).unwrap()
    }

    pub fn src_addr(&self) -> Address {
        Address::try_new(&self.buffer.as_ref()[fields::SRC_ADDR]).unwrap()
    }

    pub fn payload_type(&self) -> u16 {
        (&self.buffer.as_ref()[fields::PAYLOAD_TYPE])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn payload(&self) -> &[u8] {
        &self.buffer.as_ref()[fields::PAYLOAD]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Frame<T> {
    pub fn set_dst_addr(&mut self, addr: Address) {
        (&mut self.buffer.as_mut()[fields::DST_ADDR])
            .write(addr.as_bytes())
            .unwrap();
    }

    pub fn set_src_addr(&mut self, addr: Address) {
        (&mut self.buffer.as_mut()[fields::SRC_ADDR])
            .write(addr.as_bytes())
            .unwrap();
    }

    pub fn set_payload_type(&mut self, payload_type: u16) {
        (&mut self.buffer.as_mut()[fields::PAYLOAD_TYPE])
            .write_u16::<NetworkEndian>(payload_type)
            .unwrap();
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[fields::PAYLOAD]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_unicast() {
        let addr = Address::new([0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(addr.is_unicast());
    }

    #[test]
    fn test_is_multicast() {
        let addr = Address::new([0xFF; 6]);
        assert!(addr.is_broadcast());
    }

    #[test]
    fn test_is_broadcast() {
        let addr = Address::new([0xFF; 6]);
        assert!(addr.is_broadcast());
    }

    #[test]
    fn test_is_local() {
        let addr = Address::new([0x02, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(addr.is_local());
    }
}
