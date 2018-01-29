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

/// Safe representation of an ICMP header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repr {
    EchoReply { id: u16, seq: u16 },
    EchoRequest { id: u16, seq: u16 },
}

impl Repr {
    /// Attempts to deserialize a buffer into an ICMP packet.
    pub fn deserialize<T: AsRef<[u8]>>(packet: &Packet<T>) -> Result<Repr> {
        let header = packet.header();
        let id = || (&header[0..2]).read_u16::<NetworkEndian>().unwrap();
        let seq = || (&header[2..4]).read_u16::<NetworkEndian>().unwrap();

        match (packet.type_of(), packet.code()) {
            (0, 0) => Ok(Repr::EchoReply {
                id: id(),
                seq: seq(),
            }),
            (8, 0) => Ok(Repr::EchoRequest {
                id: id(),
                seq: seq(),
            }),
            _ => Err(Error::Encoding),
        }
    }

    /// Serializes the ICMP packet into a buffer.
    pub fn serialize<T: AsRef<[u8]> + AsMut<[u8]>>(&self, packet: &mut Packet<T>) {
        let mut echo_reply_or_request = |type_, id, seq| {
            packet.set_type(type_);
            packet.set_code(0);
            let mut header: [u8; 4] = [0; 4];
            (&mut header[0..2]).write_u16::<NetworkEndian>(id).unwrap();
            (&mut header[2..4]).write_u16::<NetworkEndian>(seq).unwrap();
            packet.set_header(header);
        };

        match *self {
            Repr::EchoReply { id, seq } => echo_reply_or_request(0, id, seq),
            Repr::EchoRequest { id, seq } => echo_reply_or_request(8, id, seq),
        }
    }
}

/// [https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol](https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol)
mod fields {
    use std;

    pub const TYPE: usize = 0;

    pub const CODE: usize = 1;

    pub const CHECKSUM: std::ops::Range<usize> = 2..4;

    pub const HEADER: std::ops::Range<usize> = 4..8;

    pub const PAYLOAD: std::ops::RangeFrom<usize> = 8..;
}

/// ICMP packet represented as a byte buffer.
#[derive(Debug)]
pub struct Packet<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> Packet<T> {
    /// Minimum size of a buffer that can encode an ICMP packet.
    pub const MIN_BUFFER_LEN: usize = 8;

    /// Wraps and represents the buffer as an ICMP packet.
    ///
    /// # Errors
    ///
    /// Causes an error if the buffer is less than Self::MIN_BUFFER_LEN bytes
    /// long. You should ensure the packet encoding is valid via is_encoding_ok()
    /// if parsing a packet from the wire. Otherwise member functions may panic.
    pub fn try_from(buffer: T) -> Result<Packet<T>> {
        if buffer.as_ref().len() < Self::MIN_BUFFER_LEN {
            return Err(Error::Buffer);
        }

        Ok(Packet { buffer })
    }

    /// Checks if the packet encoding is valid.
    pub fn is_encoding_ok(&self) -> Result<()> {
        if self.gen_checksum() != 0 {
            Err(Error::Checksum)
        } else {
            Ok(())
        }
    }

    /// Calculates a checksum for the entire packet.
    pub fn gen_checksum(&self) -> u16 {
        internet_checksum(self.buffer.as_ref())
    }

    pub fn type_of(&self) -> u8 {
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

    pub fn header(&self) -> [u8; 4] {
        let mut header: [u8; 4] = [0; 4];
        header.clone_from_slice(&self.buffer.as_ref()[fields::HEADER]);
        header
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

    pub fn set_header(&mut self, header: [u8; 4]) {
        let header_slice = &mut self.buffer.as_mut()[fields::HEADER];
        header_slice[0] = header[0];
        header_slice[1] = header[1];
        header_slice[2] = header[2];
        header_slice[3] = header[3];
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
        assert!(match Packet::try_from(&buffer[..]) {
            Err(Error::Buffer) => true,
            _ => false,
        });
    }

    #[test]
    fn test_packet_with_empty_payload() {
        let buffer: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let packet = Packet::try_from(&buffer[..]).unwrap();
        assert_eq!(packet.payload().len(), 0);
    }

    #[test]
    fn test_packet_with_invalid_checksum() {
        let buffer: [u8; 9] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        let packet = Packet::try_from(&buffer[..]).unwrap();
        assert_matches!(packet.is_encoding_ok(), Err(Error::Checksum));
    }

    #[test]
    fn test_packet_getters() {
        let buffer: [u8; 9] = [0x01, 0x02, 0xE9, 0xEf, 0x05, 0x06, 0x07, 0x08, 0x09];
        let packet = Packet::try_from(&buffer[..]).unwrap();
        assert_matches!(packet.is_encoding_ok(), Ok(_));
        assert_eq!(packet.type_of(), 1);
        assert_eq!(packet.code(), 2);
        assert_eq!(packet.checksum(), 59887);
        assert_eq!(packet.header(), [0x05, 0x06, 0x07, 0x08]);
        assert_eq!(packet.payload(), [0x09]);
    }

    #[test]
    fn test_packet_setters() {
        let mut buffer: [u8; 9] = [0; 9];

        {
            let mut packet = Packet::try_from(&mut buffer[..]).unwrap();
            packet.set_type(1);
            packet.set_code(2);
            packet.set_checksum(772);
            packet.set_header([0x05, 0x06, 0x07, 0x08]);
            packet.payload_mut()[0] = 0x09;
        }

        assert_eq!(
            &buffer[..],
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09][..]
        );
    }
}
