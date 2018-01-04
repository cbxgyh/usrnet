use std;
use std::io::Write;

use byteorder::{NetworkEndian, WriteBytesExt};

use core::addr::{Ipv4, Mac};
use core::net::{Error, Result};

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Request = 0x0001,
    Reply = 0x0002,

    #[doc(hidden)] __Nonexhaustive,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HwType {
    Ethernet = 0x0001,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtoType {
    Ethernet = 0x0800,
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
    /// You should ensure the buffer has at least buffer_len() bytes to avoid
    /// errors.
    pub fn serialize(&self, buf: &mut [u8]) -> Result<()> {
        if self.buffer_len() > buf.len() {
            return Err(Error::Overflow);
        }

        let mut buf = buf;

        match *self {
            Arp::EthernetIpv4 {
                op,
                ref source_hw_addr,
                ref source_proto_addr,
                ref target_hw_addr,
                ref target_proto_addr,
            } => {
                buf.write_u16::<NetworkEndian>(HwType::Ethernet as u16)
                    .unwrap();
                buf.write_u16::<NetworkEndian>(HwType::Ethernet as u16)
                    .unwrap();
                buf.write_u8(std::mem::size_of::<Mac>() as u8).unwrap();
                buf.write_u8(std::mem::size_of::<Ipv4>() as u8).unwrap();
                buf.write_u16::<NetworkEndian>(op as u16).unwrap();
                buf.write(source_hw_addr).unwrap();
                buf.write(source_proto_addr).unwrap();
                buf.write(target_hw_addr).unwrap();
                buf.write(target_proto_addr).unwrap();
            }
        };

        Ok(())
    }
}
