use std;
use std::io::Write;

use byteorder::{
    NetworkEndian,
    ReadBytesExt,
    WriteBytesExt,
};

use {
    Error,
    Result,
};
use core::layers::{
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arp {
    EthernetIpv4 {
        op: Op,
        source_hw_addr: EthernetAddress,
        source_proto_addr: Ipv4Address,
        target_hw_addr: EthernetAddress,
        target_proto_addr: Ipv4Address,
    },
}

impl Arp {
    /// Returns the size of the ARP packet when serialized to a buffer.
    pub fn buffer_len(&self) -> usize {
        8 + match *self {
            Arp::EthernetIpv4 { .. } => 20,
        }
    }

    /// Attempts to deserialize a buffer into an ARP packet.
    pub fn deserialize(buffer: &[u8]) -> Result<Arp> {
        if buffer.len() < 8 {
            return Err(Error::Exhausted);
        }

        let mut reader = std::io::Cursor::new(buffer);
        let hw_type = reader.read_u16::<NetworkEndian>().unwrap();
        let proto_type = reader.read_u16::<NetworkEndian>().unwrap();
        let _ = reader.read_u8().unwrap(); // Skip address sizes.
        let _ = reader.read_u8().unwrap();
        let op = reader.read_u16::<NetworkEndian>().unwrap();

        if hw_type != hw_types::ETHERNET || proto_type != proto_types::IPV4 || op == 0 || op > 2 {
            return Err(Error::Malformed);
        }

        Ok(Arp::EthernetIpv4 {
            op: if op == 1 { Op::Request } else { Op::Reply },
            source_hw_addr: EthernetAddress::try_from(&buffer[8..14]).unwrap(),
            source_proto_addr: Ipv4Address::try_from(&buffer[14..18]).unwrap(),
            target_hw_addr: EthernetAddress::try_from(&buffer[18..24]).unwrap(),
            target_proto_addr: Ipv4Address::try_from(&buffer[24..28]).unwrap(),
        })
    }

    /// Serializes the ARP packet into a buffer.
    ///
    /// You should ensure buffer has at least buffer_len() bytes to avoid errors.
    pub fn serialize(&self, buffer: &mut [u8]) -> Result<()> {
        if self.buffer_len() > buffer.len() {
            return Err(Error::Exhausted);
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
                    .write_u16::<NetworkEndian>(hw_types::ETHERNET)
                    .unwrap();
                writer
                    .write_u16::<NetworkEndian>(proto_types::IPV4)
                    .unwrap();
                writer.write_u8(6).unwrap();
                writer.write_u8(4).unwrap();
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
