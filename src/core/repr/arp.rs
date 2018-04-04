use std::io::{
    Cursor,
    Write,
};

use byteorder::{
    NetworkEndian,
    ReadBytesExt,
    WriteBytesExt,
};

use {
    Error,
    Result,
};
use core::repr::{
    EthernetAddress,
    Ipv4Address,
};

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml#arp-parameters-1
pub enum Op {
    Request = 0x0001,
    Reply = 0x0002,
}

/// https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml#arp-parameters-2
pub mod hw_types {
    pub const ETHERNET: u16 = 0x0001;
}

/// https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml#arp-parameters-3
pub mod proto_types {
    pub const IPV4: u16 = 0x0800;
}

/// An ARP packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arp {
    pub op: Op,
    pub source_hw_addr: EthernetAddress,
    pub source_proto_addr: Ipv4Address,
    pub target_hw_addr: EthernetAddress,
    pub target_proto_addr: Ipv4Address,
}

impl Arp {
    /// Returns the buffer size needed to serialize the ARP packet.
    pub fn buffer_len(&self) -> usize {
        // 8 for header + 20 for addresses.
        28
    }

    /// Tries to deserialize a buffer into an ARP packet.
    pub fn deserialize(buffer: &[u8]) -> Result<Arp> {
        if buffer.len() < 28 {
            return Err(Error::Malformed);
        }

        let hw_type = (&buffer[0 .. 2]).read_u16::<NetworkEndian>().unwrap();
        let proto_type = (&buffer[2 .. 4]).read_u16::<NetworkEndian>().unwrap();
        let op = (&buffer[6 .. 8]).read_u16::<NetworkEndian>().unwrap();

        if hw_type != hw_types::ETHERNET || proto_type != proto_types::IPV4 || op == 0 || op > 2 {
            return Err(Error::Malformed);
        }

        Ok(Arp {
            op: if op == 1 { Op::Request } else { Op::Reply },
            source_hw_addr: EthernetAddress::try_new(&buffer[8 .. 14]).unwrap(),
            source_proto_addr: Ipv4Address::try_new(&buffer[14 .. 18]).unwrap(),
            target_hw_addr: EthernetAddress::try_new(&buffer[18 .. 24]).unwrap(),
            target_proto_addr: Ipv4Address::try_new(&buffer[24 .. 28]).unwrap(),
        })
    }

    /// Serializes the ARP packet into a buffer.
    pub fn serialize(&self, buffer: &mut [u8]) -> Result<()> {
        if self.buffer_len() > buffer.len() {
            return Err(Error::Exhausted);
        }

        let mut writer = Cursor::new(buffer);
        writer
            .write_u16::<NetworkEndian>(hw_types::ETHERNET)
            .unwrap();
        writer
            .write_u16::<NetworkEndian>(proto_types::IPV4)
            .unwrap();
        writer.write_u8(6).unwrap();
        writer.write_u8(4).unwrap();
        writer.write_u16::<NetworkEndian>(self.op as u16).unwrap();
        writer.write(self.source_hw_addr.as_bytes()).unwrap();
        writer.write(self.source_proto_addr.as_bytes()).unwrap();
        writer.write(self.target_hw_addr.as_bytes()).unwrap();
        writer.write(self.target_proto_addr.as_bytes()).unwrap();

        Ok(())
    }
}
