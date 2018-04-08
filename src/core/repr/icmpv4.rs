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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    EchoReply { id: u16, seq: u16 },
    EchoRequest { id: u16, seq: u16 },
    DestinationUnreachable(DestinationUnreachable),
    TimeExceeded(TimeExceeded),
    #[doc(hidden)] ___Exhaustive,
}

/// An ICMP header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repr {
    pub message: Message,
    pub payload_len: usize,
}

impl Repr {
    /// Returns the buffer size needed to serialize the ICMP header and
    /// associated payload.
    pub fn buffer_len(&self) -> usize {
        8 + self.payload_len
    }

    /// Tries to deserialize a packet into an ICMP header.
    pub fn deserialize<T>(packet: &Packet<T>) -> Result<Repr>
    where
        T: AsRef<[u8]>,
    {
        let (id, seq) = (
            (&packet.header()[0 .. 2])
                .read_u16::<NetworkEndian>()
                .unwrap(),
            (&packet.header()[2 .. 4])
                .read_u16::<NetworkEndian>()
                .unwrap(),
        );

        let payload_len = packet.payload().len();

        match (packet._type(), packet.code()) {
            (0, 0) => Ok(Repr {
                message: Message::EchoReply { id, seq },
                payload_len,
            }),
            (8, 0) => Ok(Repr {
                message: Message::EchoRequest { id, seq },
                payload_len,
            }),
            (3, 3) => Ok(Repr {
                message: Message::DestinationUnreachable(DestinationUnreachable::PortUnreachable),
                payload_len,
            }),
            (11, 0) => Ok(Repr {
                message: Message::TimeExceeded(TimeExceeded::TTLExpired),
                payload_len,
            }),
            _ => Err(Error::Malformed),
        }
    }

    /// Serializes the ICMP header into a packet.
    ///
    /// NOTE: Use fill_checksum() on the packet before sending over the wire!
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

        fn error<T>(packet: &mut Packet<T>, type_of: u8, code: u8)
        where
            T: AsRef<[u8]> + AsMut<[u8]>,
        {
            packet.set_type(type_of);
            packet.set_code(code);
            let zeros = [0; 4];
            packet.header_mut().copy_from_slice(&zeros[..]);
        };

        match self.message {
            Message::EchoReply { id, seq } => echo(packet, 0, id, seq),
            Message::EchoRequest { id, seq } => echo(packet, 8, id, seq),
            Message::DestinationUnreachable(message) => {
                let code = match message {
                    DestinationUnreachable::PortUnreachable => 3,
                    _ => unreachable!(),
                };
                error(packet, 3, code);
            }
            Message::TimeExceeded(message) => {
                let code = match message {
                    TimeExceeded::TTLExpired => 0,
                    _ => unreachable!(),
                };
                error(packet, 11, code);
            }
            _ => unreachable!(),
        };

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

    /// Tries to create an ICMP packet from a byte buffer.
    ///
    /// NOTE: Use check_encoding() before operating on the packet if the provided
    /// buffer originates from a untrusted source such as a link.
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

    pub fn fill_checksum(&mut self) {
        self.set_checksum(0);
        let checksum = self.gen_packet_checksum();
        self.set_checksum(checksum);
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
