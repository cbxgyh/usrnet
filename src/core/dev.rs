use std;

use core::link::{
    Error as LinkError,
    Link,
};

#[derive(Debug)]
pub enum Error {
    /// Indicates a Link layer error.
    Link(LinkError),
    /// Indicates an error where the buffer is not large enough.
    Overflow,
}

impl From<LinkError> for Error {
    fn from(err: LinkError) -> Self {
        Error::Link(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A high level interface for sending frames across a link.
///
/// While a device should be backed by an underlying Link, it's main
/// responsibility is to provide a buffer allocation strategy for sending and
/// receiving frames across a Link.
pub trait Device<'a> {
    type TxBuffer: AsMut<[u8]>;

    type RxBuffer: AsRef<[u8]>;

    /// Returns a TxBuffer with at least buffer_len bytes.
    ///
    /// Callers can write to the TxBuffer ad once dropped, the TxBuffer should
    /// write to the underlying Link. Implementations must support a single
    /// outstanding TxBuffer at a time.
    ///
    /// # Panics
    ///
    /// A panic may be triggered if more than one TxBuffer exists at a time.
    fn send(&'a mut self, buffer_len: usize) -> Result<Self::TxBuffer>;

    /// Returns an RxBuffer from the underlying link.
    fn recv(&'a mut self) -> Result<Self::RxBuffer>;
}

/// A Device which reuses preallocated Tx/Rx buffers.
pub struct Standard {
    link: Box<Link>,
    tx_buffer: std::vec::Vec<u8>,
    rx_buffer: std::vec::Vec<u8>,
}

impl Standard {
    /// Creates a Standard device.
    pub fn new(link: Box<Link>) -> Result<Standard> {
        let mtu = link.get_max_transmission_unit()?;

        Ok(Standard {
            link: link,
            tx_buffer: vec![0; mtu],
            rx_buffer: vec![0; mtu],
        })
    }
}

impl<'a> Device<'a> for Standard {
    type TxBuffer = TxBuffer<'a>;

    type RxBuffer = &'a [u8];

    fn send(&'a mut self, buffer_len: usize) -> Result<Self::TxBuffer> {
        if buffer_len >= self.tx_buffer.len() {
            return Err(Error::Overflow);
        }

        Ok(TxBuffer {
            link: self.link.as_mut(),
            tx_buffer: &mut self.tx_buffer[..buffer_len],
        })
    }

    fn recv(&'a mut self) -> Result<Self::RxBuffer> {
        let buffer_len = self.link.recv(&mut self.rx_buffer)?;
        Ok(&self.rx_buffer[..buffer_len])
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
