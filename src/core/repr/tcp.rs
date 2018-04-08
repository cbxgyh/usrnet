use byteorder::{
    NetworkEndian,
    ReadBytesExt,
    WriteBytesExt,
};

use {
    Error,
    Result,
};
use core::repr::Ipv4Repr;

/// A TCP header.
///
/// Options are currently not supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repr {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset: u8,
    /// Access using the provided FLAG constants.
    pub flags: [bool; 9],
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
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

    /// Returns the length of the TCP header when serialized to a buffer.
    pub fn header_len(&self) -> usize {
        20
    }

    /// Deserializes a packet into a TCP header.
    pub fn deserialize<T>(packet: &Packet<T>) -> Repr
    where
        T: AsRef<[u8]>,
    {
        Repr {
            src_port: packet.src_port(),
            dst_port: packet.dst_port(),
            seq_num: packet.seq_num(),
            ack_num: packet.ack_num(),
            data_offset: packet.data_offset(),
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
            checksum: packet.checksum(),
            urgent_pointer: packet.urgent_pointer(),
        }
    }

    /// Serializes the TCP header into a packet and performs a checksum update.
    pub fn serialize<T>(&self, packet: &mut Packet<T>, ipv4_repr: &Ipv4Repr)
    where
        T: AsRef<[u8]> + AsMut<[u8]>,
    {
        packet.set_src_port(self.src_port);
        packet.set_dst_port(self.dst_port);
        packet.set_seq_num(self.seq_num);
        packet.set_ack_num(self.ack_num);
        packet.set_data_offset(self.data_offset);
        packet.set_window_size(self.window_size);
        packet.set_checksum(0);
        packet.set_urgent_pointer(self.urgent_pointer);

        let checksum = packet.gen_packet_checksum(ipv4_repr);
        packet.set_checksum(checksum);
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
    /// NOTE: Use check_encoding() before operating on the packet if constructing
    /// a packet via a buffer originating from an untrusted source like a link.
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

    /// Checks if the packet has a valid encoding. This may include checksum, field
    /// consistency, etc. checks.
    pub fn check_encoding(&self, ipv4_repr: &Ipv4Repr) -> Result<()> {
        if self.gen_packet_checksum(ipv4_repr) != 0 {
            Err(Error::Checksum)
        } else if ((self.data_offset() * 4) as usize) < Self::MIN_HEADER_LEN
            || (self.data_offset() as usize) * 4 >= self.buffer.as_ref().len()
        {
            Err(Error::Malformed)
        } else {
            Ok(())
        }
    }

    /// Calculates the packet checksum.
    pub fn gen_packet_checksum(&self, ipv4_repr: &Ipv4Repr) -> u16 {
        ipv4_repr.gen_checksum_with_pseudo_header(self.buffer.as_ref())
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

    pub fn seq_num(&self) -> u32 {
        (&self.buffer.as_ref()[fields::SEQ_NUM])
            .read_u32::<NetworkEndian>()
            .unwrap()
    }

    pub fn ack_num(&self) -> u32 {
        (&self.buffer.as_ref()[fields::ACK_NUM])
            .read_u32::<NetworkEndian>()
            .unwrap()
    }

    pub fn data_offset(&self) -> u8 {
        &self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][0] >> 4
    }

    pub fn ns(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][0] & 1) != 0
    }

    pub fn cwr(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 128) != 0
    }

    pub fn ece(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 64) != 0
    }

    pub fn urg(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 32) != 0
    }

    pub fn ack(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 16) != 0
    }

    pub fn psh(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 8) != 0
    }

    pub fn rst(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 4) != 0
    }

    pub fn syn(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 2) != 0
    }

    pub fn fin(&self) -> bool {
        (&self.buffer.as_ref()[fields::DATA_OFFSET_AND_FLAGS][1] & 1) != 0
    }

    pub fn window_size(&self) -> u16 {
        (&self.buffer.as_ref()[fields::WINDOW_SIZE])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn checksum(&self) -> u16 {
        (&self.buffer.as_ref()[fields::CHECKSUM])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn urgent_pointer(&self) -> u16 {
        (&self.buffer.as_ref()[fields::URGENT_POINTER])
            .read_u16::<NetworkEndian>()
            .unwrap()
    }

    pub fn payload(&self) -> &[u8] {
        let data_offset = (self.data_offset() * 4) as usize;
        &self.buffer.as_ref()[data_offset ..]
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

    pub fn set_seq_num(&mut self, seq_num: u32) {
        (&mut self.buffer.as_mut()[fields::SEQ_NUM])
            .write_u32::<NetworkEndian>(seq_num)
            .unwrap()
    }

    pub fn set_ack_num(&mut self, ack_num: u32) {
        (&mut self.buffer.as_mut()[fields::ACK_NUM])
            .write_u32::<NetworkEndian>(ack_num)
            .unwrap()
    }

    pub fn set_data_offset(&mut self, data_offset: u8) {
        let byte = &mut self.buffer.as_mut()[fields::DATA_OFFSET_AND_FLAGS][0];
        *byte &= 0b00001111;
        *byte |= data_offset << 4;
    }

    pub fn set_ns(&mut self, ns: bool) {
        self.set_flag(0, ns)
    }

    pub fn set_cwr(&mut self, cwr: bool) {
        self.set_flag(1, cwr)
    }

    pub fn set_ece(&mut self, ece: bool) {
        self.set_flag(2, ece)
    }

    pub fn set_urg(&mut self, urg: bool) {
        self.set_flag(3, urg)
    }

    pub fn set_ack(&mut self, ack: bool) {
        self.set_flag(4, ack)
    }

    pub fn set_psh(&mut self, psh: bool) {
        self.set_flag(5, psh)
    }

    pub fn set_rst(&mut self, rst: bool) {
        self.set_flag(6, rst)
    }

    pub fn set_syn(&mut self, syn: bool) {
        self.set_flag(7, syn)
    }

    pub fn set_fin(&mut self, fin: bool) {
        self.set_flag(8, fin)
    }

    fn set_flag(&mut self, flag_idx: usize, flag_val: bool) {
        let (byte_idx, bit_idx) = if flag_idx == 0 {
            (0, 0)
        } else {
            (1, 8 - flag_idx)
        };

        // (1) retrieve a reference to the byte containing the flag, (2) clear the
        // appropriate bit, and (3) set the flag bit accordingly.
        let byte = &mut self.buffer.as_mut()[fields::DATA_OFFSET_AND_FLAGS][byte_idx];
        *byte &= !(1 << bit_idx);
        if flag_val {
            *byte |= 1 << bit_idx;
        }
    }

    pub fn set_window_size(&mut self, window_size: u16) {
        (&mut self.buffer.as_mut()[fields::WINDOW_SIZE])
            .write_u16::<NetworkEndian>(window_size)
            .unwrap()
    }

    pub fn set_checksum(&mut self, checksum: u16) {
        (&mut self.buffer.as_mut()[fields::CHECKSUM])
            .write_u16::<NetworkEndian>(checksum)
            .unwrap()
    }

    pub fn set_urgent_pointer(&mut self, urgent_pointer: u16) {
        (&mut self.buffer.as_mut()[fields::URGENT_POINTER])
            .write_u16::<NetworkEndian>(urgent_pointer)
            .unwrap()
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        let data_offset = (self.data_offset() * 4) as usize;
        &mut self.buffer.as_mut()[data_offset ..]
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
        let buffer: [u8; 36] = [
            0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0xB0, 0x12, 0x00, 0x00, 0x00, 0x34, 0x51, 0xFF,
            0x43, 0x21, 0x4E, 0x2A, 0x12, 0x34, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = Packet::try_new(&buffer[..]).unwrap();

        assert_matches!(packet.check_encoding(&ipv4_repr(16)), Ok(_));
        assert_eq!(17664, packet.src_port());
        assert_eq!(20, packet.dst_port());
        assert_eq!(45074, packet.seq_num());
        assert_eq!(52, packet.ack_num());
        assert_eq!(5, packet.data_offset());
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
        assert_eq!(20010, packet.checksum());
        assert_eq!(4660, packet.urgent_pointer());
    }

    #[test]
    fn test_packet_setters() {
        let mut buffer: [u8; 36] = [0; 36];

        let mut packet = Packet::try_new(&mut buffer[..]).unwrap();
        packet.set_src_port(17664);
        packet.set_dst_port(20);
        packet.set_seq_num(45074);
        packet.set_ack_num(52);
        packet.set_data_offset(5);
        packet.set_ns(true);
        packet.set_cwr(true);
        packet.set_ece(true);
        packet.set_urg(true);
        packet.set_ack(true);
        packet.set_psh(true);
        packet.set_rst(true);
        packet.set_syn(true);
        packet.set_fin(true);
        packet.set_window_size(17185);
        packet.set_checksum(20010);
        packet.set_urgent_pointer(4660);
        packet.payload_mut()[0] = 9;

        assert_eq!(
            packet.as_ref(),
            &[
                0x45, 0x00, 0x00, 0x14, 0x00, 0x00, 0xB0, 0x12, 0x00, 0x00, 0x00, 0x34, 0x51, 0xFF,
                0x43, 0x21, 0x4E, 0x2A, 0x12, 0x34, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ][..]
        );
    }
}
