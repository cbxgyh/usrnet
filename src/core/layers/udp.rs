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
use core::layers::Ipv4Repr;

/// Safe representation of a UDP header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repr {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
}

impl Repr {
    /// Returns the UDP packet size needed to serialize this UDP header and
    /// payload.
    pub fn buffer_len(&self) -> usize {
        self.length as usize
    }

    /// Tries to deserialize a packet into a UDP header.
    pub fn deserialize<T>(packet: &Packet<T>) -> Result<Repr>
    where
        T: AsRef<[u8]>,
    {
        Ok(Repr {
            src_port: packet.src_port(),
            dst_port: packet.dst_port(),
            length: packet.length(),
        })
    }

    /// Serializes the UDP header into a packet.
    pub fn serialize<T>(&self, packet: &mut Packet<T>, ip_repr: &Ipv4Repr)
    where
        T: AsRef<[u8]> + AsMut<[u8]>,
    {
        packet.set_src_port(self.src_port);
        packet.set_dst_port(self.dst_port);
        packet.set_length(self.length);
        packet.set_checksum(0);

        let checksum = packet.gen_packet_checksum(ip_repr);
        packet.set_checksum(checksum);
    }
}

/// [https://en.wikipedia.org/wiki/User_Datagram_Protocol](https://en.wikipedia.org/wiki/User_Datagram_Protocol)
mod fields {
    use std::ops::Range;

    pub const SRC_PORT: Range<usize> = 0 .. 2;

    pub const DST_PORT: Range<usize> = 2 .. 4;

    pub const LENGTH: Range<usize> = 4 .. 6;

    pub const CHECKSUM: Range<usize> = 6 .. 8;
}

/// View of a byte buffer as a UDP packet.
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
    pub const HEADER_LEN: usize = 8;

    pub const MAX_PACKET_LEN: usize = 65535;

    /// Tries to create an ICMP packet view over a byte buffer.
    pub fn try_new(buffer: T) -> Result<Packet<T>> {
        let buffer_len = buffer.as_ref().len();

        if buffer_len < Self::buffer_len(0) || buffer_len > u16::max_value() as usize {
            Err(Error::Exhausted)
        } else {
            Ok(Packet { buffer })
        }
    }

    /// Returns the length of a UDP packet with the specified payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        8 + payload_len
    }

    /// Checks if the packet has a valid encoding. This may include checksum, field
    /// consistency, etc. checks.
    pub fn check_encoding(&self, ip_repr: &Ipv4Repr) -> Result<()> {
        // NOTE: Should enforce checksum if using IPv6, optional for IPv4.
        if self.checksum() != 0 && self.gen_packet_checksum(ip_repr) != 0 {
            Err(Error::Checksum)
        } else if self.length() as usize != self.buffer.as_ref().len() {
            Err(Error::Malformed)
        } else {
            Ok(())
        }
    }

    /// Calculates the packet checksum.
    pub fn gen_packet_checksum(&self, ip_repr: &Ipv4Repr) -> u16 {
        let mut ip_pseudo_header = [0; 12];
        (&mut ip_pseudo_header[0 .. 4]).copy_from_slice(ip_repr.src_addr.as_bytes());
        (&mut ip_pseudo_header[4 .. 8]).copy_from_slice(ip_repr.dst_addr.as_bytes());
        ip_pseudo_header[9] = ip_repr.protocol as u8;
        (&mut ip_pseudo_header[10 .. 12])
            .write_u16::<NetworkEndian>(ip_repr.payload_len)
            .unwrap();

        let iter = ip_pseudo_header
            .iter()
            .chain(self.buffer.as_ref().iter())
            .cloned();
        internet_checksum(iter)
    }

    pub fn src_port(&self) -> u16 {
        (&self.buffer.as_ref()[fields::SRC_PORT])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn dst_port(&self) -> u16 {
        (&self.buffer.as_ref()[fields::DST_PORT])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn length(&self) -> u16 {
        (&self.buffer.as_ref()[fields::LENGTH])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn checksum(&self) -> u16 {
        (&self.buffer.as_ref()[fields::CHECKSUM])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn payload(&self) -> &[u8] {
        &self.buffer.as_ref()[8 ..]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Packet<T> {
    pub fn set_src_port(&mut self, port: u16) {
        (&mut self.buffer.as_mut()[fields::SRC_PORT])
            .write_u16::<NetworkEndian>(port)
            .unwrap()
    }

    pub fn set_dst_port(&mut self, port: u16) {
        (&mut self.buffer.as_mut()[fields::DST_PORT])
            .write_u16::<NetworkEndian>(port)
            .unwrap()
    }

    pub fn set_length(&mut self, length: u16) {
        (&mut self.buffer.as_mut()[fields::LENGTH])
            .write_u16::<NetworkEndian>(length)
            .unwrap()
    }

    pub fn set_checksum(&mut self, checksum: u16) {
        (&mut self.buffer.as_mut()[fields::CHECKSUM])
            .write_u16::<NetworkEndian>(checksum)
            .unwrap()
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[8 ..]
    }
}

#[cfg(test)]
mod tests {
    use core::layers::{
        Ipv4Address,
        Ipv4Protocol,
    };

    use super::*;

    fn ip_repr(payload_len: usize) -> Ipv4Repr {
        Ipv4Repr {
            src_addr: Ipv4Address::new([0, 1, 2, 3]),
            dst_addr: Ipv4Address::new([4, 5, 6, 7]),
            protocol: Ipv4Protocol::UDP,
            payload_len: payload_len as u16,
        }
    }

    #[test]
    fn test_packet_with_buffer_less_than_min_header() {
        let buffer: [u8; 4] = [0; 4];
        let packet = Packet::try_new(&buffer[..]);
        assert_matches!(packet, Err(Error::Exhausted));
    }

    #[test]
    fn test_packet_with_invalid_checksum() {
        let buffer: [u8; 16] = [
            0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0x12, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(&ip_repr(16)), Err(Error::Checksum));
    }

    #[test]
    fn test_packet_with_inconsistent_length() {
        let buffer: [u8; 16] = [
            0x45, 0x00, 0x00, 0x14, 0x11, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(&ip_repr(16)), Err(Error::Malformed));
    }

    #[test]
    fn test_packet_getters() {
        let buffer: [u8; 16] = [
            0x04, 0x00, 0x08, 0x00, 0x00, 0x10, 0xDE, 0xBE, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(&ip_repr(16)), Ok(_));
        assert_eq!(1024, packet.src_port());
        assert_eq!(2048, packet.dst_port());
        assert_eq!(16, packet.length());
        assert_eq!(57022, packet.checksum());
        assert_eq!(8, packet.payload().len());
        assert_eq!(9, packet.payload()[0]);
    }

    #[test]
    fn test_packet_setters() {
        let mut buffer: [u8; 16] = [0; 16];

        {
            let mut packet = Packet::try_new(&mut buffer[..]).unwrap();
            packet.set_src_port(1024);
            packet.set_dst_port(2048);
            packet.set_length(16);
            packet.set_checksum(57022);
            packet.payload_mut()[0] = 9;
        }

        assert_eq!(
            &buffer[..],
            &[
                0x04, 0x00, 0x08, 0x00, 0x00, 0x10, 0xDE, 0xBE, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ][..]
        );
    }
}
