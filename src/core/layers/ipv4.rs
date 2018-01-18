use std;

use byteorder::{
    NetworkEndian,
    ReadBytesExt,
};

use core::layers::{
    Error,
    Result,
};

/// [IPv4 address](https://en.wikipedia.org/wiki/IPv4) in network byte order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address([u8; 4]);

impl Address {
    /// Creates an IPv4 address from a network byte order buffer.
    pub fn new(addr: [u8; 4]) -> Address {
        Address(addr)
    }

    /// Creates an IPv4 address from a network byte order slice.
    pub fn try_from(addr: &[u8]) -> Result<Address> {
        if addr.len() != 4 {
            return Err(Error::Buffer);
        }

        let mut _addr: [u8; 4] = [0; 4];
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
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl std::str::FromStr for Address {
    type Err = ();

    /// Parses an Ipv4 address from an A.B.C.D style string.
    fn from_str(addr: &str) -> std::result::Result<Address, Self::Err> {
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

/// IPv4 packet represented as a byte buffer.
pub struct Packet<T>
where
    T: AsRef<[u8]>,
{
    buffer: T,
}

impl<T> Packet<T>
where
    T: AsRef<[u8]>,
{
    /// Wraps and represents the buffer as an IPv4 packet.
    ///
    /// Might result in an error if the encoding is invalid. This can happen for
    /// example if the header length is longer than the buffer.
    pub fn new(buffer: T) -> Result<Packet<T>> {
        let buffer_len = buffer.as_ref().len();

        if buffer_len < Self::buffer_len(0) {
            return Err(Error::Buffer);
        }

        let packet = Packet { buffer };

        if packet.header_len() < 20 || buffer_len < packet.header_len() as usize
            || buffer_len != packet.packet_len() as usize
        {
            return Err(Error::Encoding);
        }

        Ok(packet)
    }

    /// Returns the length of a IPv4 packet with no options and the payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        20 + payload_len
    }

    pub fn ip_version(&self) -> u8 {
        (self.buffer.as_ref()[0] & 0xF0) >> 4 as u8
    }

    pub fn header_len(&self) -> u8 {
        (self.buffer.as_ref()[0] & 0x0F) * 4 as u8
    }

    pub fn dscp(&self) -> u8 {
        (self.buffer.as_ref()[1] & 0xFC) >> 2
    }

    pub fn ecn(&self) -> u8 {
        self.buffer.as_ref()[1] & 0x03
    }

    pub fn packet_len(&self) -> u16 {
        (&self.buffer.as_ref()[2..4])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn identification(&self) -> u16 {
        (&self.buffer.as_ref()[4..6])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn flags(&self) -> u8 {
        (self.buffer.as_ref()[6] & 0xC0) as u8
    }

    pub fn fragment_offset(&self) -> u16 {
        (&self.buffer.as_ref()[6..8])
            .read_u16::<NetworkEndian>()
            .unwrap() & 0x5FFF
    }

    pub fn ttl(&self) -> u8 {
        self.buffer.as_ref()[8]
    }

    pub fn protocol(&self) -> u8 {
        self.buffer.as_ref()[9]
    }

    pub fn header_checksum(&self) -> u16 {
        (&self.buffer.as_ref()[10..12])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn src_addr(&self) -> Address {
        Address::try_from(&self.buffer.as_ref()[12..16]).unwrap()
    }

    pub fn dst_addr(&self) -> Address {
        Address::try_from(&self.buffer.as_ref()[16..20]).unwrap()
    }

    /// Returns an immutable view of the payload.
    pub fn payload(&self) -> &[u8] {
        &self.buffer.as_ref()[20..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_buffer_less_than_min_header() {
        let buffer: [u8; 1] = [0; 1];
        assert!(match Packet::new(&buffer[..]) {
            Err(Error::Buffer) => true,
            _ => false,
        });
    }

    #[test]
    fn test_packet_header_less_than_min_header() {
        let buffer: [u8; 20] = [
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(match Packet::new(&buffer[..]) {
            Err(Error::Encoding) => true,
            _ => false,
        });
    }

    #[test]
    fn test_packet_buffer_less_than_header() {
        // 0x0F = 15 words = 60 bytes
        let buffer: [u8; 20] = [
            0x0F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(match Packet::new(&buffer[..]) {
            Err(Error::Encoding) => true,
            _ => false,
        });
    }

    #[test]
    fn test_packet_buffer_less_than_packet() {
        let buffer: [u8; 20] = [
            0x0F, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(match Packet::new(&buffer[..]) {
            Err(Error::Encoding) => true,
            _ => false,
        });
    }

    #[test]
    fn test_packet_with_valid_buffer() {
        let buffer: [u8; 36] = [
            0x45, 0x11, 0x00, 0x24, 0xFF, 0xFF, 0x01, 0x01, 0x02, 0x03, 0x00, 0x04, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = Packet::new(&buffer[..]).unwrap();
        assert_eq!(4, packet.ip_version());
        assert_eq!(20, packet.header_len());
        assert_eq!(4, packet.dscp());
        assert_eq!(1, packet.ecn());
        assert_eq!(36, packet.packet_len());
        assert_eq!(65535, packet.identification());
        assert_eq!(0, packet.flags());
        assert_eq!(257, packet.fragment_offset());
        assert_eq!(2, packet.ttl());
        assert_eq!(3, packet.protocol());
        assert_eq!(4, packet.header_checksum());
        assert_eq!(16, packet.payload().len());
        assert_eq!(1, packet.payload()[0]);
    }
}
