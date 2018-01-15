use std;

use core::link::{
    Error as LinkError,
    Link,
};
use core::repr::{
    Ipv4,
    Mac,
};

#[derive(Debug)]
pub enum Error {
    /// Indicates a Link layer error.
    Link(LinkError),
    /// Indicates an error where a buffer was not large enough.
    Overflow,
    /// Indicates a situation with an empty link.
    Nothing,
}

impl From<LinkError> for Error {
    fn from(err: LinkError) -> Self {
        Error::Link(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A high level interface for sending frames across a link.
///
/// While a device should be backed by an underlying link, it's main job
/// is to provide a buffer allocation strategy for sending and receiving frames
/// across a link and associate the link with a set of addresses.
pub trait Device<'a> {
    type TxBuffer: AsMut<[u8]>;

    type RxBuffer: AsRef<[u8]>;

    /// Returns a TxBuffer with at least buffer_len bytes.
    ///
    /// Callers can write to the TxBuffer. Once dropped, the TxBuffer should
    /// write to the underlying link. Implementations must support a single
    /// outstanding TxBuffer at a time.
    ///
    /// # Panics
    ///
    /// A panic may be triggered if more than one TxBuffer exists at a time.
    fn send(&'a mut self, buffer_len: usize) -> Result<Self::TxBuffer>;

    /// Returns a frame from the underlying link in an RxBuffer.
    fn recv(&'a mut self) -> Result<Self::RxBuffer>;

    /// Returns the Ipv4 address associated with the device.
    fn get_ipv4_addr(&self) -> Ipv4;

    /// Returns the ethernet address associated with the device.
    fn get_ethernet_addr(&self) -> Mac;
}

/// A Device which reuses preallocated Tx/Rx buffers.
pub struct Standard<T: Link> {
    link: T,
    tx_buffer: std::vec::Vec<u8>,
    rx_buffer: std::vec::Vec<u8>,
    ipv4_addr: Ipv4,
    eth_addr: Mac,
}

impl<T: Link> Standard<T> {
    /// Creates a Standard device.
    pub fn new(link: T, ipv4_addr: Ipv4, eth_addr: Mac) -> Result<Standard<T>> {
        let mtu = link.get_max_transmission_unit()?;

        Ok(Standard {
            link: link,
            tx_buffer: vec![0; mtu],
            rx_buffer: vec![0; mtu],
            ipv4_addr: ipv4_addr,
            eth_addr: eth_addr,
        })
    }
}

impl<'a, T: Link> Device<'a> for Standard<T> {
    type TxBuffer = TxBuffer<'a>;

    type RxBuffer = &'a [u8];

    fn send(&'a mut self, buffer_len: usize) -> Result<Self::TxBuffer> {
        if buffer_len >= self.tx_buffer.len() {
            return Err(Error::Overflow);
        }

        Ok(TxBuffer {
            link: &mut self.link,
            tx_buffer: &mut self.tx_buffer[..buffer_len],
        })
    }

    fn recv(&'a mut self) -> Result<Self::RxBuffer> {
        let buffer_len = self.link.recv(&mut self.rx_buffer)?;
        if buffer_len == 0 {
            return Err(Error::Nothing);
        }
        Ok(&self.rx_buffer[..buffer_len])
    }

    fn get_ipv4_addr(&self) -> Ipv4 {
        self.ipv4_addr
    }

    fn get_ethernet_addr(&self) -> Mac {
        self.eth_addr
    }
}

/// A TxBuffer for a Device which uses a heap allocated Vec for storage.
pub struct TxBuffer<'a> {
    link: &'a mut Link,
    tx_buffer: &'a mut [u8],
}

impl<'a> AsRef<[u8]> for TxBuffer<'a> {
    fn as_ref(&self) -> &[u8] {
        self.tx_buffer.as_ref()
    }
}

impl<'a> AsMut<[u8]> for TxBuffer<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.tx_buffer.as_mut()
    }
}

impl<'a> Drop for TxBuffer<'a> {
    fn drop(&mut self) {
        self.link.send(self.tx_buffer).unwrap();
    }
}
