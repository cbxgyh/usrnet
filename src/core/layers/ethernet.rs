use std;
use std::io::Write;

use byteorder::{
    NetworkEndian,
    ReadBytesExt,
    WriteBytesExt,
};

use core::layers::{
    Error,
    Result,
};

/// [MAC address](https://en.wikipedia.org/wiki/MAC_address) in network byte order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address([u8; 6]);

impl Address {
    pub const BROADCAST: Address = Address([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);

    /// Creates a MAC address from a network byte order buffer.
    pub fn new(addr: [u8; 6]) -> Address {
        Address(addr)
    }

    /// Creates a MAC address from a network byte order slice.
    pub fn try_from(addr: &[u8]) -> Result<Address> {
        if addr.len() != 6 {
            return Err(Error::Buffer);
        }

        let mut _addr: [u8; 6] = [0; 6];
        _addr.clone_from_slice(addr);
        Ok(Address(_addr))
    }

    /// Returns a reference to the network byte order representation of the address.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5],
        )
    }
}

impl std::str::FromStr for Address {
    type Err = ();

    /// Parses a MAC address from an A:B:C:D:E:F style string.
    fn from_str(addr: &str) -> std::result::Result<Address, Self::Err> {
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
pub mod types {
    pub const IPV4: u16 = 0x800;

    pub const ARP: u16 = 0x806;
}

mod fields {
    use std;

    pub const DST_ADDR: std::ops::Range<usize> = 0..6;

    pub const SRC_ADDR: std::ops::Range<usize> = 6..12;

    pub const PAYLOAD_TYPE: std::ops::Range<usize> = 12..14;

    pub const PAYLOAD: std::ops::RangeFrom<usize> = 14..;
}

/// Ethernet frame represented as a byte buffer.
#[derive(Debug)]
pub struct Frame<T>
where
    T: AsRef<[u8]>,
{
    buffer: T,
}

impl<T> Frame<T>
where
    T: AsRef<[u8]>,
{
    pub const MIN_BUFFER_SIZE: usize = 14;

    /// Wraps and represents the buffer as an Ethernet frame.
    ///
    /// # Errors
    ///
    /// Causes an error if the buffer is less than MIN_BUFFER_SIZE bytes long.
    pub fn try_from(buffer: T) -> Result<Frame<T>> {
        if buffer.as_ref().len() < Self::MIN_BUFFER_SIZE {
            return Err(Error::Buffer);
        }

        Ok(Frame { buffer })
    }

    /// Returns the length of an Ethernet frame with the specified payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        Self::MIN_BUFFER_SIZE + payload_len
    }

    /// Gets the hardware destination address.
    pub fn dst_addr(&self) -> Address {
        Address::try_from(&self.buffer.as_ref()[fields::DST_ADDR]).unwrap()
    }

    /// Gets the hardware source address.
    pub fn src_addr(&mut self) -> Address {
        Address::try_from(&self.buffer.as_ref()[fields::SRC_ADDR]).unwrap()
    }

    /// Returns the payload type of the frame or an error containing the unknown code.
    pub fn payload_type(&self) -> u16 {
        (&self.buffer.as_ref()[fields::PAYLOAD_TYPE])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    /// Returns an immutable view of the payload.
    pub fn payload(&self) -> &[u8] {
        &self.buffer.as_ref()[fields::PAYLOAD]
    }
}

impl<T> Frame<T>
where
    T: AsRef<[u8]> + AsMut<[u8]>,
{
    /// Sets the hardware destination address.
    pub fn set_dst_addr(&mut self, addr: Address) {
        (&mut self.buffer.as_mut()[fields::DST_ADDR])
            .write(addr.as_bytes())
            .unwrap();
    }

    /// Sets the hardware source address.
    pub fn set_src_addr(&mut self, addr: Address) {
        (&mut self.buffer.as_mut()[fields::SRC_ADDR])
            .write(addr.as_bytes())
            .unwrap();
    }

    /// Sets the payload type.
    pub fn set_payload_type(&mut self, payload_type: u16) {
        (&mut self.buffer.as_mut()[fields::PAYLOAD_TYPE])
            .write_u16::<NetworkEndian>(payload_type)
            .unwrap();
    }

    /// Returns a mutable view of the payload.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[fields::PAYLOAD]
    }
}
