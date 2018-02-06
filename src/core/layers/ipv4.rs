use std;
use std::io::Write;

use byteorder::{
    NetworkEndian,
    ReadBytesExt,
    WriteBytesExt,
};

use core::check::internet_checksum;
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

impl std::hash::Hash for Address {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash_slice(&self.0[..], h)
    }
}

/// [https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml](https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml)
pub mod types {
    pub const ICMP: u8 = 1;
}

pub mod flags {
    pub const DONT_FRAGMENT: u8 = 0b00000010;

    pub const NOT_LAST: u8 = 0b00000001;
}

/// [https://en.wikipedia.org/wiki/IPv4](https://en.wikipedia.org/wiki/IPv4)
mod fields {
    use std;

    pub const IP_VERSION_AND_HEADER_LEN: usize = 0;

    pub const DSCP_AND_ECN: usize = 1;

    pub const PACKET_LEN: std::ops::Range<usize> = 2..4;

    pub const IDENTIFICATION: std::ops::Range<usize> = 4..6;

    pub const FLAGS: usize = 6;

    pub const FRAG_OFFSET: std::ops::Range<usize> = 6..8;

    pub const TTL: usize = 8;

    pub const PROTOCOL: usize = 9;

    pub const CHECKSUM: std::ops::Range<usize> = 10..12;

    pub const SRC_ADDR: std::ops::Range<usize> = 12..16;

    pub const DST_ADDR: std::ops::Range<usize> = 16..20;
}

/// IPv4 packet represented as a byte buffer.
#[derive(Debug)]
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
    /// Wraps and represents the buffer as an Ethernet frame.
    ///
    /// # Errors
    ///
    /// Causes an error if the buffer is too small or too large. You should
    /// ensure the packet encoding is valid via is_encoding_ok() if parsing
    /// a packet from the wire. Otherwise member functions may panic.
    pub fn try_from(buffer: T) -> Result<Packet<T>> {
        let buffer_len = buffer.as_ref().len();

        if buffer_len < Self::buffer_len(0) || buffer_len > u16::max_value() as usize {
            return Err(Error::Buffer);
        }

        Ok(Packet { buffer })
    }

    /// Checks if the packet encoding is valid.
    pub fn is_encoding_ok(&self) -> Result<()> {
        if self.ip_version() != 4 || ((self.header_len() * 4) as usize) > self.buffer.as_ref().len()
            || (self.packet_len() as usize) > self.buffer.as_ref().len()
        {
            return Err(Error::Encoding);
        }

        if self.gen_header_checksum() != 0 {
            return Err(Error::Checksum);
        }

        Ok(())
    }

    /// Returns the length of a IPv4 packet with no options and the payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        20 + payload_len
    }

    /// Calculates a checksum for the entire header.
    pub fn gen_header_checksum(&self) -> u16 {
        let header_len = (self.header_len() * 4) as usize;
        internet_checksum(&self.buffer.as_ref()[..header_len])
    }

    pub fn ip_version(&self) -> u8 {
        (self.buffer.as_ref()[fields::IP_VERSION_AND_HEADER_LEN] & 0xF0) >> 4
    }

    pub fn header_len(&self) -> u8 {
        (self.buffer.as_ref()[fields::IP_VERSION_AND_HEADER_LEN] & 0x0F)
    }

    pub fn dscp(&self) -> u8 {
        (self.buffer.as_ref()[fields::DSCP_AND_ECN] & 0xFC) >> 2
    }

    pub fn ecn(&self) -> u8 {
        self.buffer.as_ref()[fields::DSCP_AND_ECN] & 0x03
    }

    pub fn packet_len(&self) -> u16 {
        (&self.buffer.as_ref()[fields::PACKET_LEN])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn identification(&self) -> u16 {
        (&self.buffer.as_ref()[fields::IDENTIFICATION])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn flags(&self) -> u8 {
        (self.buffer.as_ref()[fields::FLAGS] & 0xE0) >> 5
    }

    pub fn fragment_offset(&self) -> u16 {
        let frag_offset_slice = &self.buffer.as_ref()[fields::FRAG_OFFSET];
        let mut frag_offset_only: [u8; 2] = [0; 2];
        frag_offset_only[0] = frag_offset_slice[0] & 0x1F; // Clear flags!
        frag_offset_only[1] = frag_offset_slice[1];
        (&frag_offset_only[..]).read_u16::<NetworkEndian>().unwrap()
    }

    pub fn ttl(&self) -> u8 {
        self.buffer.as_ref()[fields::TTL]
    }

    pub fn protocol(&self) -> u8 {
        self.buffer.as_ref()[fields::PROTOCOL]
    }

    pub fn header_checksum(&self) -> u16 {
        (&self.buffer.as_ref()[fields::CHECKSUM])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn src_addr(&self) -> Address {
        Address::try_from(&self.buffer.as_ref()[fields::SRC_ADDR]).unwrap()
    }

    pub fn dst_addr(&self) -> Address {
        Address::try_from(&self.buffer.as_ref()[fields::DST_ADDR]).unwrap()
    }

    /// Returns an immutable view of the payload.
    pub fn payload(&self) -> &[u8] {
        &self.buffer.as_ref()[(self.header_len() * 4) as usize..]
    }
}

impl<T> Packet<T>
where
    T: AsRef<[u8]> + AsMut<[u8]>,
{
    pub fn set_ip_version(&mut self, version: u8) {
        self.buffer.as_mut()[fields::IP_VERSION_AND_HEADER_LEN] &= !0xF0;
        self.buffer.as_mut()[fields::IP_VERSION_AND_HEADER_LEN] |= version << 4;
    }

    pub fn set_header_len(&mut self, header_len: u8) {
        self.buffer.as_mut()[fields::IP_VERSION_AND_HEADER_LEN] &= !0x0F;
        self.buffer.as_mut()[fields::IP_VERSION_AND_HEADER_LEN] |= header_len & 0x0F;
    }

    pub fn set_dscp(&mut self, dscp: u8) {
        self.buffer.as_mut()[fields::DSCP_AND_ECN] &= !0xFC;
        self.buffer.as_mut()[fields::DSCP_AND_ECN] |= dscp << 2;
    }

    pub fn set_ecn(&mut self, ecn: u8) {
        self.buffer.as_mut()[fields::DSCP_AND_ECN] &= !0x03;
        self.buffer.as_mut()[fields::DSCP_AND_ECN] |= ecn & 0x03;
    }

    pub fn set_packet_len(&mut self, packet_len: u16) {
        (&mut self.buffer.as_mut()[fields::PACKET_LEN])
            .write_u16::<NetworkEndian>(packet_len)
            .unwrap()
    }

    pub fn set_identification(&mut self, id: u16) {
        (&mut self.buffer.as_mut()[fields::IDENTIFICATION])
            .write_u16::<NetworkEndian>(id)
            .unwrap()
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.buffer.as_mut()[fields::FLAGS] &= 0x1F;
        self.buffer.as_mut()[fields::FLAGS] |= flags << 5
    }

    pub fn set_fragment_offset(&mut self, frag_offset: u16) {
        let flags = self.flags();
        (&mut self.buffer.as_mut()[fields::FRAG_OFFSET])
            .write_u16::<NetworkEndian>(frag_offset)
            .unwrap();
        self.set_flags(flags);
    }

    pub fn set_ttl(&mut self, ttl: u8) {
        self.buffer.as_mut()[fields::TTL] = ttl;
    }

    pub fn set_protocol(&mut self, protocol: u8) {
        self.buffer.as_mut()[fields::PROTOCOL] = protocol;
    }

    pub fn set_header_checksum(&mut self, header_checksum: u16) {
        (&mut self.buffer.as_mut()[fields::CHECKSUM])
            .write_u16::<NetworkEndian>(header_checksum)
            .unwrap()
    }

    pub fn set_src_addr(&mut self, addr: Address) {
        (&mut self.buffer.as_mut()[fields::SRC_ADDR])
            .write(addr.as_bytes())
            .unwrap();
    }

    pub fn set_dst_addr(&mut self, addr: Address) {
        (&mut self.buffer.as_mut()[fields::DST_ADDR])
            .write(addr.as_bytes())
            .unwrap();
    }

    /// Returns a mutable view of the payload.
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let header_len = self.header_len();
        &mut self.buffer.as_mut()[(header_len * 4) as usize..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_with_buffer_less_than_min_header() {
        let buffer: [u8; 19] = [0; 19];
        let packet = Packet::try_from(&buffer[..]);
        assert_matches!(packet, Err(Error::Buffer));
    }

    #[test]
    fn test_packet_with_header_len_greater_than_buffer_len() {
        let buffer: [u8; 20] = [
            0x4F, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_from(&buffer[..]).unwrap();
        assert_matches!(packet.is_encoding_ok(), Err(Error::Encoding));
    }

    #[test]
    fn test_packet_with_packet_len_greater_than_buffer_len() {
        let buffer: [u8; 20] = [
            0x45, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_from(&buffer[..]).unwrap();
        assert_matches!(packet.is_encoding_ok(), Err(Error::Encoding));
    }

    #[test]
    fn test_packet_with_invalid_checksum() {
        let buffer: [u8; 20] = [
            0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_from(&buffer[..]).unwrap();
        assert_matches!(packet.is_encoding_ok(), Err(Error::Checksum));
    }

    #[test]
    fn test_packet_getters() {
        let buffer: [u8; 40] = [
            0x46, 0x11, 0x00, 0x28, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0xC6, 0xAD, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = Packet::try_from(&buffer[..]).unwrap();
        assert_matches!(packet.is_encoding_ok(), Ok(_));
        assert_eq!(4, packet.ip_version());
        assert_eq!(6, packet.header_len());
        assert_eq!(4, packet.dscp());
        assert_eq!(1, packet.ecn());
        assert_eq!(40, packet.packet_len());
        assert_eq!(65535, packet.identification());
        assert_eq!(7, packet.flags());
        assert_eq!(257, packet.fragment_offset());
        assert_eq!(2, packet.ttl());
        assert_eq!(3, packet.protocol());
        assert_eq!(50861, packet.header_checksum());
        assert_eq!(Address([1, 2, 3, 4]), packet.src_addr());
        assert_eq!(Address([5, 6, 7, 8]), packet.dst_addr());
        assert_eq!(16, packet.payload().len());
        assert_eq!(9, packet.payload()[0]);
    }

    #[test]
    fn test_packet_setters() {
        let mut buffer: [u8; 40] = [0; 40];

        {
            let mut packet = Packet::try_from(&mut buffer[..]).unwrap();
            packet.set_ip_version(4);
            packet.set_header_len(6);
            packet.set_dscp(4);
            packet.set_ecn(1);
            packet.set_packet_len(40);
            packet.set_identification(65535);
            packet.set_flags(7);
            packet.set_fragment_offset(257);
            packet.set_ttl(2);
            packet.set_protocol(3);
            packet.set_header_checksum(4);
            packet.set_src_addr(Address([1, 2, 3, 4]));
            packet.set_dst_addr(Address([5, 6, 7, 8]));
            packet.payload_mut()[0] = 0x09;
        }

        assert_eq!(
            &buffer[..],
            &[
                0x46, 0x11, 0x00, 0x28, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0x00, 0x04, 0x01, 0x02,
                0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..]
        );
    }
}
