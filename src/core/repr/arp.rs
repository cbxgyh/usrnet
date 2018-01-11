use std;
use std::io::Write;

use byteorder::{
    NetworkEndian,
    WriteBytesExt,
};

use core::repr::{
    Error,
    Ipv4,
    Mac,
    Result,
};

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml#arp-parameters-1
pub enum Op {
    Request = 0x0001,
    Reply = 0x0002,

    #[doc(hidden)] __Nonexhaustive,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml#arp-parameters-2
pub enum HwType {
    Ethernet = 0x0001,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml#arp-parameters-3
pub enum ProtoType {
    Ipv4 = 0x0800,
}

pub enum Arp {
    EthernetIpv4 {
        op: Op,
        source_hw_addr: Mac,
        source_proto_addr: Ipv4,
        target_hw_addr: Mac,
        target_proto_addr: Ipv4,
    },
}

impl Arp {
    /// Returns the size of the ARP packet when serialized to a buffer.
    pub fn buffer_len(&self) -> usize {
        8 + match *self {
            Arp::EthernetIpv4 { .. } => 20,
        }
    }

    /// Serializes the ARP packet into a buffer.
    ///
    /// You should ensure buffer has at least buffer_len() bytes to avoid errors.
    pub fn serialize(&self, buffer: &mut [u8]) -> Result<()> {
        if self.buffer_len() > buffer.len() {
            return Err(Error::Buffer);
        }

        match *self {
            Arp::EthernetIpv4 {
                op,
                ref source_hw_addr,
                ref source_proto_addr,
                ref target_hw_addr,
                ref target_proto_addr,
            } => {
                let mut writer = std::io::Cursor::new(buffer);
                writer
                    .write_u16::<NetworkEndian>(HwType::Ethernet as u16)
                    .unwrap();
                writer
                    .write_u16::<NetworkEndian>(ProtoType::Ipv4 as u16)
                    .unwrap();
                writer.write_u8(6 as u8).unwrap();
                writer.write_u8(4 as u8).unwrap();
                writer.write_u16::<NetworkEndian>(op as u16).unwrap();
                writer.write(source_hw_addr.as_bytes()).unwrap();
                writer.write(source_proto_addr.as_bytes()).unwrap();
                writer.write(target_hw_addr.as_bytes()).unwrap();
                writer.write(target_proto_addr.as_bytes()).unwrap();
            }
        };

        Ok(())
    }
}
