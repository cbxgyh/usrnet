use byteorder::{
    ByteOrder,
    NetworkEndian,
};

use core::repr::Ipv4Repr;
use {
    Error,
    Result,
};

/// A TCP header.
///
/// Options are currently not supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repr {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    /// Access using the provided FLAG constants.
    pub flags: [bool; 9],
    pub window_size: u16,
    pub urgent_pointer: u16,
    pub max_segment_size: Option<u16>,
}

impl Repr {
    pub const FLAG_NS: usize = 0;

    pub const FLAG_CWR: usize = 1;

    pub const FLAG_ECE: usize = 2;

    pub const FLAG_URG: usize = 3;

    pub const FLAG_ACK: usize = 4;

    pub const FLAG_PSH: usize = 5;

    pub const FLAG_RST: usize = 6;

    pub const FLAG_SYN: usize = 7;

    pub const FLAG_FIN: usize = 8;

    /// Returns the length of the TCP header (including options!) when
    /// serialized to a buffer.
    pub fn header_len(&self) -> usize {
        20 + if self.max_segment_size.is_some() {
            4
        } else {
            0
        }
    }

    /// Deserializes a packet into a TCP header.
    pub fn deserialize<T>(packet: &Packet<T>) -> Repr
    where
        T: AsRef<[u8]>,
    {
        let options_iter = TcpOptionIter::new(packet.options());

        Repr {
            src_port: packet.src_port(),
            dst_port: packet.dst_port(),
            seq_num: packet.seq_num(),
            ack_num: packet.ack_num(),
            flags: [
                packet.ns(),
                packet.cwr(),
                packet.ece(),
                packet.urg(),
                packet.ack(),
                packet.psh(),
                packet.rst(),
                packet.syn(),
                packet.fin(),
            ],
            window_size: packet.window_size(),
            urgent_pointer: packet.urgent_pointer(),
            max_segment_size: options_iter
                .filter_map(|option| match option {
                    TcpOption::MaxSegmentSize(mss) => Some(mss),
                    _ => None,
                })
                .next(),
        }
    }

    /// Serializes the TCP header into a packet.
    pub fn serialize<T>(&self, packet: &mut Packet<T>) -> Result<()>
    where
        T: AsRef<[u8]> + AsMut<[u8]>,
    {
        if self.header_len() > packet.as_ref().len() {
            return Err(Error::Exhausted);
        }

        packet.set_src_port(self.src_port);
        packet.set_dst_port(self.dst_port);
        packet.set_seq_num(self.seq_num);
        packet.set_ack_num(self.ack_num);

        // When using options, make sure the header length is a multiple of 32 bits
        // using the NOP option.
        let data_offset = 5 + if self.max_segment_size.is_some() {
            1
        } else {
            0
        };
        packet.set_data_offset(data_offset);

        packet.set_ns(self.flags[Self::FLAG_NS]);
        packet.set_cwr(self.flags[Self::FLAG_CWR]);
        packet.set_ece(self.flags[Self::FLAG_ECE]);
        packet.set_urg(self.flags[Self::FLAG_URG]);
        packet.set_ack(self.flags[Self::FLAG_ACK]);
        packet.set_psh(self.flags[Self::FLAG_PSH]);
        packet.set_rst(self.flags[Self::FLAG_RST]);
        packet.set_syn(self.flags[Self::FLAG_SYN]);
        packet.set_fin(self.flags[Self::FLAG_FIN]);
        packet.set_window_size(self.window_size);
        packet.set_checksum(0);
        packet.set_urgent_pointer(self.urgent_pointer);

        // Ok for now... in the future we may support arbitrary options on the
        // Repr and should support generic serialization of options.
        match self.max_segment_size {
            Some(mss) => {
                let options = packet.options_mut();
                options[0] = 2;
                options[1] = 4;
                NetworkEndian::write_u16(&mut options[2 .. 4], mss);
            }
            _ => {}
        };

        Ok(())
    }
}

/// [https://en.wikipedia.org/wiki/Transmission_Control_Protocol#TCP_segment_structure](https://en.wikipedia.org/wiki/Transmission_Control_Protocol#TCP_segment_structure)
mod fields {
    use std::ops::Range;

    pub const SRC_PORT: Range<usize> = 0 .. 2;

    pub const DST_PORT: Range<usize> = 2 .. 4;

    pub const SEQ_NUM: Range<usize> = 4 .. 8;

    pub const ACK_NUM: Range<usize> = 8 .. 12;

    pub const DATA_OFFSET_AND_FLAGS: Range<usize> = 12 .. 14;

    pub const WINDOW_SIZE: Range<usize> = 14 .. 16;

    pub const CHECKSUM: Range<usize> = 16 .. 18;

    pub const URGENT_POINTER: Range<usize> = 18 .. 20;
}

/// A TCP option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TcpOption<'a> {
    EOL,
    NoOp,
    MaxSegmentSize(u16),
    Unknown { kind: u8, payload: &'a [u8] },
}

/// An iterator that produces TcpOptions from a buffer.
pub struct TcpOptionIter<'a> {
    options: &'a [u8],
    position: usize,
}

impl<'a> Iterator for TcpOptionIter<'a> {
    type Item = TcpOption<'a>;

    fn next(&mut self) -> Option<TcpOption<'a>> {
        if self.position == self.options.len() {
            return None;
        }

        let kind = self.options[self.position];
        let (option, len) = match kind {
            0 => (TcpOption::EOL, 1),
            1 => (TcpOption::NoOp, 1),
            _ => {
                if self.position + 2 > self.options.len() {
                    // No space for length field!
                    return None;
                }

                let len = self.options[self.position + 1] as usize;

                if self.position + len > self.options.len() {
                    // Length exceeds buffer!
                    return None;
                }

                let payload = &self.options[self.position + 2 .. self.position + len];

                match (kind, len) {
                    (2, 4) => {
                        let mss = NetworkEndian::read_u16(payload);
                        (TcpOption::MaxSegmentSize(mss), 4)
                    }
                    _ => (TcpOption::Unknown { kind, payload }, len),
                }
            }
        };

        self.position += len;

        Some(option)
    }
}

impl<'a> TcpOptionIter<'a> {
    /// Creates a new TCP options iterator from a buffer.
    pub fn new(options: &'a [u8]) -> TcpOptionIter<'a> {
        TcpOptionIter {
            options,
            position: 0,
        }
    }
}

/// View of a byte buffer as a TCP packet.
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

    /// Tries to create an TCP packet from a byte buffer.
    ///
    /// NOTE: Use check_encoding() before operating on the packet if
    /// constructing a packet via a buffer originating from an untrusted
    /// source like a link.
    pub fn try_new(buffer: T) -> Result<Packet<T>> {
        if buffer.as_ref().len() < Self::MIN_HEADER_LEN {
            Err(Error::Exhausted)
        } else {
            Ok(Packet { buffer })
        }
    }

    /// Returns the length of a TCP packet with no options and the specified
    /// payload size.
    pub fn buffer_len(payload_len: usize) -> usize {
        20 + payload_len
    }

    /// Checks if the packet has a valid encoding. This may include checksum,
    /// field consistency, etc. checks.
    pub fn check_encoding(&self, ipv4_repr: &Ipv4Repr) -> Result<()> {
        if self.gen_packet_checksum(ipv4_repr) != 0 {
            Err(Error::Checksum)
        } else if ((self.data_offset() * 4) as usize) < Self::MIN_HEADER_LEN
            || (self.data_offset() as usize) * 4 > self.as_ref().len()
        {
            Err(Error::Malformed)
        } else {
            Ok(())
        }
    }

    /// Calculates the packet checksum.
    pub fn gen_packet_checksum(&self, ipv4_repr: &Ipv4Repr) -> u16 {
        ipv4_repr.gen_checksum_with_pseudo_header(self.as_ref())
    }

    pub fn src_port(&self) -> u16 {
        NetworkEndian::read_u16(&self.as_ref()[fields::SRC_PORT])
    }

    pub fn dst_port(&self) -> u16 {
        NetworkEndian::read_u16(&self.as_ref()[fields::DST_PORT])
    }

    pub fn seq_num(&self) -> u32 {
        NetworkEndian::read_u32(&self.as_ref()[fields::SEQ_NUM])
    }

    pub fn ack_num(&self) -> u32 {
        NetworkEndian::read_u32(&self.as_ref()[fields::ACK_NUM])
    }

    pub fn data_offset(&self) -> u8 {
        &self.as_ref()[fields::DATA_OFFSET_AND_FLAGS][0] >> 4
    }

    pub fn ns(&self) -> bool {
        self.flag(8)
    }

    pub fn cwr(&self) -> bool {
        self.flag(7)
    }

    pub fn ece(&self) -> bool {
        self.flag(6)
    }

    pub fn urg(&self) -> bool {
        self.flag(5)
    }

    pub fn ack(&self) -> bool {
        self.flag(4)
    }

    pub fn psh(&self) -> bool {
        self.flag(3)
    }

    pub fn rst(&self) -> bool {
        self.flag(2)
    }

    pub fn syn(&self) -> bool {
        self.flag(1)
    }

    pub fn fin(&self) -> bool {
        self.flag(0)
    }

    fn flag(&self, index: usize) -> bool {
        let field = NetworkEndian::read_u16(&self.as_ref()[fields::DATA_OFFSET_AND_FLAGS]);
        (field & (1 << index)) != 0
    }

    pub fn window_size(&self) -> u16 {
        NetworkEndian::read_u16(&self.as_ref()[fields::WINDOW_SIZE])
    }

    pub fn checksum(&self) -> u16 {
        NetworkEndian::read_u16(&self.as_ref()[fields::CHECKSUM])
    }

    pub fn urgent_pointer(&self) -> u16 {
        NetworkEndian::read_u16(&self.as_ref()[fields::URGENT_POINTER])
    }

    pub fn options(&self) -> &[u8] {
        let data_offset = (self.data_offset() * 4) as usize;
        &self.as_ref()[Self::MIN_HEADER_LEN .. data_offset]
    }

    pub fn payload(&self) -> &[u8] {
        let data_offset = (self.data_offset() * 4) as usize;
        &self.as_ref()[data_offset ..]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> Packet<T> {
    pub fn set_src_port(&mut self, port: u16) {
        NetworkEndian::write_u16(&mut self.as_mut()[fields::SRC_PORT], port);
    }

    pub fn set_dst_port(&mut self, port: u16) {
        NetworkEndian::write_u16(&mut self.as_mut()[fields::DST_PORT], port);
    }

    pub fn set_seq_num(&mut self, seq_num: u32) {
        NetworkEndian::write_u32(&mut self.as_mut()[fields::SEQ_NUM], seq_num);
    }

    pub fn set_ack_num(&mut self, ack_num: u32) {
        NetworkEndian::write_u32(&mut self.as_mut()[fields::ACK_NUM], ack_num);
    }

    pub fn set_data_offset(&mut self, data_offset: u8) {
        let mut field = self.as_ref()[fields::DATA_OFFSET_AND_FLAGS][0];
        field &= 0x0F;
        field |= data_offset << 4;
        self.as_mut()[fields::DATA_OFFSET_AND_FLAGS][0] = field;
    }

    pub fn set_ns(&mut self, ns: bool) {
        self.set_flag(8, ns)
    }

    pub fn set_cwr(&mut self, cwr: bool) {
        self.set_flag(7, cwr)
    }

    pub fn set_ece(&mut self, ece: bool) {
        self.set_flag(6, ece)
    }

    pub fn set_urg(&mut self, urg: bool) {
        self.set_flag(5, urg)
    }

    pub fn set_ack(&mut self, ack: bool) {
        self.set_flag(4, ack)
    }

    pub fn set_psh(&mut self, psh: bool) {
        self.set_flag(3, psh)
    }

    pub fn set_rst(&mut self, rst: bool) {
        self.set_flag(2, rst)
    }

    pub fn set_syn(&mut self, syn: bool) {
        self.set_flag(1, syn)
    }

    pub fn set_fin(&mut self, fin: bool) {
        self.set_flag(0, fin)
    }

    fn set_flag(&mut self, index: usize, enabled: bool) {
        let mut field = NetworkEndian::read_u16(&self.as_ref()[fields::DATA_OFFSET_AND_FLAGS]);
        field = if enabled {
            field | (1 << index)
        } else {
            field & !(1 << index)
        };
        NetworkEndian::write_u16(&mut self.as_mut()[fields::DATA_OFFSET_AND_FLAGS], field);
    }

    pub fn set_window_size(&mut self, window_size: u16) {
        NetworkEndian::write_u16(&mut self.as_mut()[fields::WINDOW_SIZE], window_size);
    }

    pub fn set_checksum(&mut self, checksum: u16) {
        NetworkEndian::write_u16(&mut self.as_mut()[fields::CHECKSUM], checksum);
    }

    pub fn set_urgent_pointer(&mut self, urgent_pointer: u16) {
        NetworkEndian::write_u16(&mut self.as_mut()[fields::URGENT_POINTER], urgent_pointer);
    }

    pub fn options_mut(&mut self) -> &mut [u8] {
        let data_offset = (self.data_offset() * 4) as usize;
        &mut self.as_mut()[Self::MIN_HEADER_LEN .. data_offset]
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        let data_offset = (self.data_offset() * 4) as usize;
        &mut self.as_mut()[data_offset ..]
    }

    pub fn fill_checksum(&mut self, ipv4_repr: &Ipv4Repr) {
        self.set_checksum(0);
        let checksum = self.gen_packet_checksum(ipv4_repr);
        self.set_checksum(checksum);
    }
}

#[cfg(test)]
mod tests {
    use core::repr::{
        Ipv4Address,
        Ipv4Protocol,
    };

    use super::*;

    fn ipv4_repr(payload_len: usize) -> Ipv4Repr {
        Ipv4Repr {
            src_addr: Ipv4Address::new([0, 1, 2, 3]),
            dst_addr: Ipv4Address::new([4, 5, 6, 7]),
            protocol: Ipv4Protocol::TCP,
            payload_len: payload_len as u16,
        }
    }

    #[test]
    fn test_packet_with_buffer_less_than_min_header() {
        let buffer: [u8; 19] = [0; 19];
        let packet = Packet::try_new(&buffer[..]);
        assert_matches!(packet, Err(Error::Exhausted));
    }

    #[test]
    fn test_packet_with_invalid_checksum() {
        let buffer: [u8; 36] = [
            0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0x12, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x9C, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(&ipv4_repr(16)), Err(Error::Checksum));
    }

    #[test]
    fn test_packet_with_invalid_data_offset() {
        let buffer: [u8; 36] = [
            0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0x12, 0x34, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
            0x00, 0x00, 0x8C, 0x91, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let packet = Packet::try_new(&buffer[..]).unwrap();
        assert_matches!(packet.check_encoding(&ipv4_repr(16)), Err(Error::Malformed));
    }

    #[test]
    fn test_packet_getters() {
        let buffer: [u8; 40] = [
            0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0xB0, 0x12, 0x00, 0x00, 0x00, 0x34, 0x61, 0xFF,
            0x43, 0x21, 0x3B, 0x26, 0x12, 0x34, 0x02, 0x04, 0x01, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = Packet::try_new(&buffer[..]).unwrap();

        assert_matches!(packet.check_encoding(&ipv4_repr(16)), Ok(_));
        assert_eq!(17664, packet.src_port());
        assert_eq!(20, packet.dst_port());
        assert_eq!(45074, packet.seq_num());
        assert_eq!(52, packet.ack_num());
        assert_eq!(6, packet.data_offset());
        assert_eq!(17185, packet.window_size());
        assert_eq!(true, packet.ns());
        assert_eq!(true, packet.cwr());
        assert_eq!(true, packet.ece());
        assert_eq!(true, packet.urg());
        assert_eq!(true, packet.ack());
        assert_eq!(true, packet.psh());
        assert_eq!(true, packet.rst());
        assert_eq!(true, packet.syn());
        assert_eq!(true, packet.fin());
        assert_eq!(15142, packet.checksum());
        assert_eq!(4660, packet.urgent_pointer());

        let repr = Repr::deserialize(&packet);

        assert_eq!(
            repr,
            Repr {
                src_port: 17664,
                dst_port: 20,
                seq_num: 45074,
                ack_num: 52,
                flags: [true; 9],
                window_size: 17185,
                urgent_pointer: 4660,
                max_segment_size: Some(256),
            }
        );
    }

    #[test]
    fn test_packet_setters() {
        let repr = Repr {
            src_port: 17664,
            dst_port: 20,
            seq_num: 45074,
            ack_num: 52,
            flags: [true; 9],
            window_size: 17185,
            urgent_pointer: 4660,
            max_segment_size: Some(256),
        };

        assert_eq!(24, repr.header_len());

        let mut buffer: [u8; 40] = [0; 40];

        let mut packet = Packet::try_new(&mut buffer[..]).unwrap();
        repr.serialize(&mut packet).unwrap();
        packet.payload_mut()[0] = 9;
        packet.fill_checksum(&ipv4_repr(16));

        assert_eq!(
            packet.as_ref(),
            &[
                0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0xB0, 0x12, 0x00, 0x00, 0x00, 0x34, 0x61, 0xFF,
                0x43, 0x21, 0x3B, 0x26, 0x12, 0x34, 0x02, 0x04, 0x01, 0x00, 0x09, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..]
        );
    }

    #[test]
    fn test_options_iterator() {
        let mut buffer: [u8; 20] = [0, 1, 3, 4, 4, 5, 2, 4, 1, 1, 0, 1, 1, 1, 5, 99, 2, 4, 1, 1];
        let options: Vec<_> = TcpOptionIter::new(&mut buffer).collect();

        assert_eq!(
            options,
            vec![
                TcpOption::EOL,
                TcpOption::NoOp,
                TcpOption::Unknown {
                    kind: 3,
                    payload: &[4, 5],
                },
                TcpOption::MaxSegmentSize(257),
                TcpOption::EOL,
                TcpOption::NoOp,
                TcpOption::NoOp,
                TcpOption::NoOp,
            ]
        );
    }
}
