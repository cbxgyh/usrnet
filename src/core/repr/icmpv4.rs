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

/// Safe representation of an ICMP header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repr {
    EchoReply {
        id: u16,
        seq: u16,
    },
    EchoRequest {
        id: u16,
        seq: u16,
    },
    DestinationUnreachable {
        reason: DestinationUnreachable,
        ipv4_header_len: usize,
    },
    TimeExceeded {
        reason: TimeExceeded,
        ipv4_header_len: usize,
    },
    #[doc(hidden)] ___Exhaustive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationUnreachable {
    PortUnreachable,
    #[doc(hidden)] ___Exhaustive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeExceeded {
    TTLExpired,
    #[doc(hidden)] ___Exhaustive,
}

impl Repr {
    /// Returns the ICMP packet size needed to serialize this ICMP
    /// representation.
    pub fn buffer_len(&self) -> usize {
        match *self {
            Repr::DestinationUnreachable {
                ipv4_header_len, ..
            } => ipv4_header_len + 16,
            Repr::TimeExceeded {
                ipv4_header_len, ..
            } => ipv4_header_len + 16,
            _ => 8,
        }
    }

    /// Tries to deserialize a packet into an ICMP representation.
    pub fn deserialize<T>(packet: &Packet<T>) -> Result<Repr>
    where
        T: AsRef<[u8]>,
    {
        fn echo_id_seq<T>(packet: &Packet<T>) -> (u16, u16)
        where
            T: AsRef<[u8]>,
        {
            (
                (&packet.header()[0 .. 2])
                    .read_u16::<NetworkEndian>()
                    .unwrap(),
                (&packet.header()[2 .. 4])
                    .read_u16::<NetworkEndian>()
                    .unwrap(),
            )
        }

        fn ipv4_header_len<T>(packet: &Packet<T>) -> Result<usize>
        where
            T: AsRef<[u8]>,
        {
            // IP header (20 bytes) + payload (8 bytes) minimum!
            if packet.payload().len() < 28 {
                Err(Error::Malformed)
            } else {
                Ok(packet.payload().len() - 8)
            }
        }

        match (packet._type(), packet.code()) {
            (0, 0) => {
                let (id, seq) = echo_id_seq(packet);
                Ok(Repr::EchoReply { id, seq })
            }
            (8, 0) => {
                let (id, seq) = echo_id_seq(packet);
                Ok(Repr::EchoRequest { id, seq })
            }
            (3, 3) => Ok(Repr::DestinationUnreachable {
                reason: DestinationUnreachable::PortUnreachable,
                ipv4_header_len: ipv4_header_len(packet)?,
            }),
            (11, 0) => Ok(Repr::TimeExceeded {
                reason: TimeExceeded::TTLExpired,
                ipv4_header_len: ipv4_header_len(packet)?,
            }),
            _ => Err(Error::Malformed),
        }
    }

    /// Serializes the ICMP representation into a packet.
    pub fn serialize<T>(&self, packet: &mut Packet<T>) -> Result<()>
    where
        T: AsRef<[u8]> + AsMut<[u8]>,
    {
        fn echo<T>(packet: &mut Packet<T>, type_of: u8, id: u16, seq: u16)
        where
            T: AsRef<[u8]> + AsMut<[u8]>,
        {
            packet.set_type(type_of);
            packet.set_code(0);

            (&mut packet.header_mut()[0 .. 2])
                .write_u16::<NetworkEndian>(id)
                .unwrap();
            (&mut packet.header_mut()[2 .. 4])
                .write_u16::<NetworkEndian>(seq)
                .unwrap();
        };

        fn error<T>(
            packet: &mut Packet<T>,
            ipv4_header_len: usize,
            type_of: u8,
            code: u8,
        ) -> Result<()>
        where
            T: AsRef<[u8]> + AsMut<[u8]>,
        {
            if packet.payload().len() != ipv4_header_len + 8 {
                return Err(Error::Malformed);
            }
            packet.set_type(type_of);
            packet.set_code(code);
            let zeros = [0; 4];
            packet.header_mut().copy_from_slice(&zeros[..]);
            Ok(())
        };

        match *self {
            Repr::EchoReply { id, seq } => echo(packet, 0, id, seq),
            Repr::EchoRequest { id, seq } => echo(packet, 8, id, seq),
            Repr::DestinationUnreachable {
                reason,
                ipv4_header_len,
            } => {
                let code = match reason {
                    DestinationUnreachable::PortUnreachable => 3,
                    _ => unreachable!(),
                };
                error(packet, ipv4_header_len, 3, code)?;
            }
            Repr::TimeExceeded {
                reason,
                ipv4_header_len,
            } => {
                let code = match reason {
                    TimeExceeded::TTLExpired => 0,
                    _ => unreachable!(),
                };
                error(packet, ipv4_header_len, 11, code)?;
            }
            _ => unreachable!(),
        };

        packet.set_checksum(0);
        let checksum = packet.gen_packet_checksum();
        packet.set_checksum(checksum);

        Ok(())
    }
}

/// [https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol](https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol)
mod fields {
    use std::ops::{
        Range,
        RangeFrom,
    };

    pub const TYPE: usize = 0;

    pub const CODE: usize = 1;

    pub const CHECKSUM: Range<usize> = 2 .. 4;

    pub const HEADER: Range<usize> = 4 .. 8;

    pub const PAYLOAD: RangeFrom<usize> = 8 ..;
}

/// View of a byte buffer as an ICMP packet.
#[derive(Debug)]
pub struct Packet<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> Packet<T> {
    pub const HEADER_LEN: usize = 8;

    pub const MAX_PACKET_LEN: usize = 65535;

    /// Tries to create an ICMP packet view over a byte buffer.
    pub fn try_new(buffer: T) -> Result<Packet<T>> {
        if buffer.as_ref().len() < Self::HEADER_LEN || buffer.as_ref().len() > Self::MAX_PACKET_LEN
        {
            Err(Error::Exhausted)
        } else {
            Ok(Packet { buffer })
        }
    }

    /// Returns the length of an ICMP packet with the specified payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        Self::HEADER_LEN + payload_len
    }

    /// Checks if the packet has a valid encoding. This may include checksum, field
    /// consistency, etc. checks.
    pub fn check_encoding(&self) -> Result<()> {
        if self.gen_packet_checksum() != 0 {
            Err(Error::Checksum)
        } else {
            Ok(())
        }
    }

    /// Calculates the packet checksum.
    pub fn gen_packet_checksum(&self) -> u16 {
        internet_checksum(self.buffer.as_ref())
    }

    pub fn _type(&self) -> u8 {
        self.buffer.as_ref()[fields::TYPE]
    }

    pub fn code(&self) -> u8 {
        self.buffer.as_ref()[fields::CODE]
    }

    pub fn checksum(&self) -> u16 {
        (&self.buffer.as_ref()[fields::CHECKSUM])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn header(&self) -> &[u8] {
        &self.buffer.as_ref()[fields::HEADER]
    }

    pub fn payload(&self) -> &[u8] {
        &self.buffer.as_ref()[fields::PAYLOAD]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Packet<T> {
    pub fn set_type(&mut self, type_of: u8) {
        self.buffer.as_mut()[fields::TYPE] = type_of
    }

    pub fn set_code(&mut self, code: u8) {
        self.buffer.as_mut()[fields::CODE] = code;
    }

    pub fn set_checksum(&mut self, checksum: u16) {
        (&mut self.buffer.as_mut()[fields::CHECKSUM])
            .write_u16::<NetworkEndian>(checksum)
            .unwrap()
    }

    pub fn header_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[fields::HEADER]
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.buffer.as_mut()[fields::PAYLOAD]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_buffer_too_small() {
        let buffer: [u8; 7] = [0; 7];
        assert!(match Packet::try_new(&buffer[..]) {
            Err(Error::Exhausted) => true,
            _ => false,
        });
    }

    #[test]
    fn test_packet_with_empty_payload() {
        let buffer: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_eq!(packet.payload().len(), 0);
    }

    #[test]
    fn test_packet_with_invalid_checksum() {
        let buffer: [u8; 9] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Err(Error::Checksum));
    }

    #[test]
    fn test_packet_getters() {
        let buffer: [u8; 9] = [0x01, 0x02, 0xE9, 0xEf, 0x05, 0x06, 0x07, 0x08, 0x09];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(), Ok(_));
        assert_eq!(packet._type(), 1);
        assert_eq!(packet.code(), 2);
        assert_eq!(packet.checksum(), 59887);
        assert_eq!(packet.header(), [0x05, 0x06, 0x07, 0x08]);
        assert_eq!(packet.payload(), [0x09]);
    }

    #[test]
    fn test_packet_setters() {
        let mut buffer: [u8; 9] = [0; 9];

        {
            let mut packet = Packet::try_new(&mut buffer[..]).unwrap();
            packet.set_type(1);
            packet.set_code(2);
            packet.set_checksum(772);
            packet
                .header_mut()
                .copy_from_slice(&[0x05, 0x06, 0x07, 0x08]);
            packet.payload_mut()[0] = 0x09;
        }

        assert_eq!(
            &buffer[..],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09][..]
        );
    }
}
