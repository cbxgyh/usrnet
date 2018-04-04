use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult,
};
use std::io::Write;
use std::ops::Deref;
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
use core::check::internet_checksum;

/// [IPv4 address](https://en.wikipedia.org/wiki/IPv4) in network byte order.
/// See [this](https://en.wikipedia.org/wiki/Classful_network) for a description
/// of IPv4 address classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Address([u8; 4]);

impl Address {
    /// Creates an IPv4 address from a network byte order buffer.
    pub fn new(addr: [u8; 4]) -> Address {
        Address(addr)
    }

    /// Tries to creates an IPv4 address from a network byte order slice.
    pub fn try_new(addr: &[u8]) -> Result<Address> {
        if addr.len() != 4 {
            return Err(Error::Exhausted);
        }

        let mut _addr: [u8; 4] = [0; 4];
        _addr.clone_from_slice(addr);
        Ok(Address(_addr))
    }

    /// Returns a reference to the network byte order representation of the
    /// address.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns an integer representation of the address in host byte order.
    pub fn as_int(&self) -> u32 {
        (&self.0[..]).read_u32::<NetworkEndian>().unwrap()
    }

    // Checks if this is a unicast address.
    pub fn is_unicast(&self) -> bool {
        !(self.is_multicast() || self.is_reserved())
    }

    // Checks if this is a multicast address.
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0b11100000) == 0b11100000
    }

    // Checks if this is a reserved address.
    pub fn is_reserved(&self) -> bool {
        (self.0[0] & 0b11110000) == 0b11110000
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl From<u32> for Address {
    fn from(addr: u32) -> Address {
        let mut bytes = [0; 4];
        (&mut bytes[..]).write_u32::<NetworkEndian>(addr).unwrap();
        Address(bytes)
    }
}

impl FromStr for Address {
    type Err = ();

    /// Parses an Ipv4 address from an A.B.C.D style string.
    fn from_str(addr: &str) -> StdResult<Address, Self::Err> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AddressCidr {
    address: Address,
    subnet_len: u32,
}

impl AddressCidr {
    /// Creates an IPv4 address with a subnet mask.
    ///
    /// # Panics
    ///
    /// Causes a panic if the subnet mask is longer than 32 bits.
    pub fn new(address: Address, subnet_len: usize) -> AddressCidr {
        assert!(subnet_len <= 32);

        AddressCidr {
            address,
            subnet_len: subnet_len as u32,
        }
    }

    /// Checks if the address is a member of the subnet.
    pub fn is_member(&self, address: Address) -> bool {
        let mask = !(0xFFFFFFFF >> self.subnet_len);
        (address.as_int() & mask) == (self.address.as_int() & mask)
    }

    /// Checks if the address is a broadcast address for the subnet.
    pub fn is_broadcast(&self, address: Address) -> bool {
        address == self.broadcast()
    }

    /// Creates an IPv4 broadcast address for the subnet.
    pub fn broadcast(&self) -> Address {
        let mask = !(0xFFFFFFFF >> self.subnet_len);
        let addr = (self.address.as_int() & mask) | (!mask);
        Address::from(addr)
    }
}

impl Deref for AddressCidr {
    type Target = Address;

    fn deref(&self) -> &Address {
        &self.address
    }
}

impl Display for AddressCidr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}/{}", self.address, self.subnet_len)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
/// A set of supported protocols over IPv4.
pub enum Protocol {
    ICMP = protocols::ICMP,
    UDP = protocols::UDP,
    TCP = protocols::TCP,
    #[doc(hidden)] __Nonexhaustive,
}

/// An IPv4 header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repr {
    pub src_addr: Address,
    pub dst_addr: Address,
    pub protocol: Protocol,
    pub payload_len: u16,
}

impl Repr {
    /// Returns the buffer size needed to serialize the IPv4 header and
    /// associated payload.
    pub fn buffer_len(&self) -> usize {
        Packet::<&[u8]>::MIN_HEADER_LEN + (self.payload_len as usize)
    }

    /// Tries to deserialize a packet into an IPv4 header.
    pub fn deserialize<T>(packet: &Packet<T>) -> Result<Repr>
    where
        T: AsRef<[u8]>,
    {
        Ok(Repr {
            src_addr: packet.src_addr(),
            dst_addr: packet.dst_addr(),
            protocol: match packet.protocol() {
                protocols::ICMP => Protocol::ICMP,
                protocols::UDP => Protocol::UDP,
                _ => return Err(Error::Malformed),
            },
            payload_len: packet.payload().len() as u16,
        })
    }

    /// Serializes the IPv4 header into a packet and performs a checksum update.
    pub fn serialize<T>(&self, packet: &mut Packet<T>)
    where
        T: AsRef<[u8]> + AsMut<[u8]>,
    {
        packet.set_ip_version(4);
        packet.set_header_len(5);
        packet.set_dscp(0);
        packet.set_ecn(0);
        packet.set_packet_len(20 + self.payload_len as u16);
        packet.set_identification(0);
        packet.set_flags(flags::DONT_FRAGMENT);
        packet.set_fragment_offset(0);
        packet.set_ttl(64);
        packet.set_protocol(self.protocol as u8);
        packet.set_header_checksum(0);
        packet.set_src_addr(self.src_addr);
        packet.set_dst_addr(self.dst_addr);

        let checksum = packet.gen_header_checksum();
        packet.set_header_checksum(checksum);
    }

    /// Generates a checksum for the byte buffer, using a pseudo-header
    /// corresponding to this IP header.
    pub fn gen_checksum_with_pseudo_header(&self, buffer: &[u8]) -> u16 {
        let mut ip_pseudo_header = [0; 12];
        (&mut ip_pseudo_header[0 .. 4]).copy_from_slice(self.src_addr.as_bytes());
        (&mut ip_pseudo_header[4 .. 8]).copy_from_slice(self.dst_addr.as_bytes());
        ip_pseudo_header[9] = self.protocol as u8;
        (&mut ip_pseudo_header[10 .. 12])
            .write_u16::<NetworkEndian>(self.payload_len)
            .unwrap();

        let iter = ip_pseudo_header
            .iter()
            .chain(buffer.as_ref().iter())
            .cloned();
        internet_checksum(iter)
    }
}

/// [https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml](https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml)
pub mod protocols {
    pub const ICMP: u8 = 1;

    pub const TCP: u8 = 6;

    pub const UDP: u8 = 17;
}

pub mod flags {
    pub const DONT_FRAGMENT: u8 = 0b00000010;

    pub const NOT_LAST: u8 = 0b00000001;
}

/// [https://en.wikipedia.org/wiki/IPv4](https://en.wikipedia.org/wiki/IPv4)
mod fields {
    use std::ops::Range;

    pub const IP_VERSION_AND_HEADER_LEN: usize = 0;

    pub const DSCP_AND_ECN: usize = 1;

    pub const PACKET_LEN: Range<usize> = 2 .. 4;

    pub const IDENTIFICATION: Range<usize> = 4 .. 6;

    pub const FLAGS: usize = 6;

    pub const FRAG_OFFSET: Range<usize> = 6 .. 8;

    pub const TTL: usize = 8;

    pub const PROTOCOL: usize = 9;

    pub const CHECKSUM: Range<usize> = 10 .. 12;

    pub const SRC_ADDR: Range<usize> = 12 .. 16;

    pub const DST_ADDR: Range<usize> = 16 .. 20;
}

/// View of a byte buffer as an IPv4 packet.
#[derive(Debug)]
pub struct Packet<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> AsRef<[u8]> for Packet<T> {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> AsMut<[u8]> for Packet<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.buffer.as_mut()
    }
}

impl<T: AsRef<[u8]>> Packet<T> {
    pub const MIN_HEADER_LEN: usize = 20;

    /// Tries to create an IPv4 packet from a byte buffer.
    ///
    /// NOTE: Use check_encoding() before operating on the packet if the provided
    /// buffer originates from a untrusted source such as a link.
    pub fn try_new(buffer: T) -> Result<Packet<T>> {
        if buffer.as_ref().len() < Self::MIN_HEADER_LEN {
            Err(Error::Exhausted)
        } else {
            Ok(Packet { buffer })
        }
    }

    /// Returns the length of an IPv4 packet with the specified payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        20 + payload_len
    }

    /// Checks if the packet has a valid encoding. This may include checksum, field
    /// consistency, etc. checks.
    pub fn check_encoding(&self) -> Result<()> {
        if (self.packet_len() as usize) > self.buffer.as_ref().len()
            || ((self.header_len() * 4) as usize) < Self::MIN_HEADER_LEN
            || ((self.header_len() * 4) as usize) > self.buffer.as_ref().len()
            || self.ip_version() != 4
        {
            Err(Error::Malformed)
        } else if self.gen_header_checksum() != 0 {
            Err(Error::Checksum)
        } else {
            Ok(())
        }
    }

    /// Calculates the header checksum.
    pub fn gen_header_checksum(&self) -> u16 {
        let header_len = (self.header_len() * 4) as usize;
        internet_checksum(&self.buffer.as_ref()[.. header_len])
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
        Address::try_new(&self.buffer.as_ref()[fields::SRC_ADDR]).unwrap()
    }

    pub fn dst_addr(&self) -> Address {
        Address::try_new(&self.buffer.as_ref()[fields::DST_ADDR]).unwrap()
    }

    pub fn payload(&self) -> &[u8] {
        let header_len = (self.header_len() * 4) as usize;
        let packet_len = self.packet_len() as usize;
        &self.buffer.as_ref()[header_len .. packet_len]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Packet<T> {
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

    pub fn payload_mut(&mut self) -> &mut [u8] {
        let header_len = (self.header_len() * 4) as usize;
        let packet_len = self.packet_len() as usize;
        &mut self.buffer.as_mut()[header_len .. packet_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_unicast() {
        let addr = Address::new([0x00, 0x00, 0x00, 0x00]);
        assert!(addr.is_unicast());
        let addr = Address::new([0x80, 0x00, 0x00, 0x00]);
        assert!(addr.is_unicast());
        let addr = Address::new([0xC0, 0x00, 0x00, 0x00]);
        assert!(addr.is_unicast());
    }

    #[test]
    fn test_is_multicast() {
        let addr = Address::new([0xE0, 0x00, 0x00, 0x00]);
        assert!(addr.is_multicast());
    }

    #[test]
    fn test_is_reserved() {
        let addr = Address::new([0xF0, 0x00, 0x00, 0x00]);
        assert!(addr.is_reserved());
    }

    #[test]
    fn test_addr_cidr_is_member() {
        let addr = AddressCidr::new(Address::new([0x12, 0x30, 0x00, 0x00]), 4);

        assert!(!addr.is_member(Address::new([0x00, 0x00, 0x00, 0x00])));
        assert!(addr.is_member(Address::new([0x10, 0x00, 0x00, 0x00])));
        assert!(addr.is_member(Address::new([0x12, 0x30, 0x00, 0x00])));
        assert!(addr.is_member(Address::new([0x12, 0x34, 0x00, 0x00])));
        assert!(addr.is_member(Address::new([0x1F, 0xFF, 0xFF, 0xFF])));
    }

    #[test]
    fn test_addr_broadcast() {
        let addr = AddressCidr::new(Address::new([0x12, 0x30, 0x00, 0x00]), 4);
        assert_eq!(addr.broadcast(), Address::new([0x1F, 0xFF, 0xFF, 0xFF]));
    }

    #[test]
    fn test_addr_is_broadcast() {
        let addr = AddressCidr::new(Address::new([0x12, 0x30, 0x00, 0x00]), 4);

        assert!(!addr.is_broadcast(Address::new([0x0F, 0xFF, 0xFF, 0xFF])));
        assert!(addr.is_broadcast(Address::new([0x1F, 0xFF, 0xFF, 0xFF])));
    }

    #[test]
    fn test_packet_with_buffer_less_than_min_header() {
        let buffer: [u8; 19] = [0; 19];
        let packet = Packet::try_new(&buffer[..]);
        assert_matches!(packet, Err(Error::Exhausted));
    }

    #[test]
    fn test_packet_with_invalid_packet_len() {
        let buffer: [u8; 42] = [
            0x41, 0x11, 0x00, 0x13, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Err(Error::Malformed));

        let buffer: [u8; 42] = [
            0x41, 0x11, 0x00, 0xFF, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Err(Error::Malformed));
    }

    #[test]
    fn test_packet_with_invalid_header_len() {
        let buffer: [u8; 42] = [
            0x41, 0x11, 0x00, 0x28, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Err(Error::Malformed));

        let buffer: [u8; 42] = [
            0x4F, 0x11, 0x00, 0x28, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Err(Error::Malformed));
    }

    #[test]
    fn test_packet_with_invalid_ip_version() {
        let buffer: [u8; 42] = [
            0x41, 0x11, 0x00, 0x28, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Err(Error::Malformed));
    }

    #[test]
    fn test_packet_with_invalid_checksum() {
        let buffer: [u8; 42] = [
            0x46, 0x11, 0x00, 0x28, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Err(Error::Checksum));
    }

    #[test]
    fn test_packet_getters() {
        let buffer: [u8; 42] = [
            0x46, 0x11, 0x00, 0x28, 0xFF, 0xFF, 0xE1, 0x01, 0x02, 0x03, 0xC6, 0xAD, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Ok(_));
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
        let mut buffer: [u8; 42] = [0; 42];

        {
            let mut packet = Packet::try_new(&mut buffer[..]).unwrap();
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
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..]
        );
    }
}
