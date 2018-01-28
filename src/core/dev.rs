use std;

use core::link::{
    Error as LinkError,
    Link,
};
use core::layers::{
    EthernetAddress,
    Ipv4Address,
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

    /// Returns a zero'd TxBuffer with at least buffer_len bytes.
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
    fn get_ipv4_addr(&self) -> Ipv4Address;

    /// Returns the ethernet address associated with the device.
    fn get_ethernet_addr(&self) -> EthernetAddress;
}

/// A Device which reuses preallocated Tx/Rx buffers.
pub struct Standard<T: Link> {
    link: T,
    tx_buffer: std::vec::Vec<u8>,
    rx_buffer: std::vec::Vec<u8>,
    ipv4_addr: Ipv4Address,
    eth_addr: EthernetAddress,
}

impl<T: Link> Standard<T> {
    /// Creates a Standard device.
    pub fn new(link: T, ipv4_addr: Ipv4Address, eth_addr: EthernetAddress) -> Result<Standard<T>> {
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

        let buffer = &mut self.tx_buffer[..buffer_len];
        for b in buffer.iter_mut() {
            *b = 0;
        }

        Ok(TxBuffer {
            link: &mut self.link,
            tx_buffer: buffer,
        })
    }

    fn recv(&'a mut self) -> Result<Self::RxBuffer> {
        let buffer_len = self.link.recv(&mut self.rx_buffer)?;
        if buffer_len == 0 {
            return Err(Error::Nothing);
        }
        Ok(&self.rx_buffer[..buffer_len])
    }

    fn get_ipv4_addr(&self) -> Ipv4Address {
        self.ipv4_addr
    }

    fn get_ethernet_addr(&self) -> EthernetAddress {
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

#[cfg(test)]
mod tests {
    use core::link::MockLink;
    use super::*;

    fn new_test_dev(link: MockLink) -> Standard<MockLink> {
        Standard::new(
            link,
            Ipv4Address::new([10, 0, 0, 1]),
            EthernetAddress::new([0, 1, 2, 3, 4, 5]),
        ).unwrap()
    }

    #[test]
    fn test_send() {
        let mut link = MockLink::new();

        let mtu = link.method_get_max_transmission_unit().set_result(Ok(1500));
        link.set_get_max_transmission_unit(mtu);

        let send = link.method_send()
            .first_call()
            .set_result(Ok(()))
            .second_call()
            .set_result(Ok(()));
        link.set_send(send);

        let mut dev = new_test_dev(link);

        {
            let mut buffer = dev.send(1).unwrap();
            buffer.as_mut()[0] = 9;
        }

        {
            // Ensure buffer is 0'd on subsequent sends...
            let buffer = dev.send(2).unwrap();
            assert_eq!(buffer.as_ref(), [0, 0]);
        }
    }

    #[test]
    fn test_send_overflow() {
        let mut link = MockLink::new();

        let mtu = link.method_get_max_transmission_unit()
            .return_result_of(|| Ok(100));
        link.set_get_max_transmission_unit(mtu);

        let mut dev = new_test_dev(link);

        assert!(match dev.send(101) {
            Err(Error::Overflow) => true,
            _ => false,
        });
    }

    #[test]
    fn test_recv() {
        let mut link = MockLink::new();

        let mtu = link.method_get_max_transmission_unit()
            .return_result_of(|| Ok(1500));
        link.set_get_max_transmission_unit(mtu);

        let recv = link.method_recv().set_result(Ok(100));
        link.set_recv(recv);

        let mut dev = new_test_dev(link);

        assert!(match dev.recv() {
            Ok(ref buffer) => buffer.len() == 100,
            _ => false,
        });
    }

    #[test]
    fn test_recv_link_errors() {
        let mut link = MockLink::new();

        let mtu = link.method_get_max_transmission_unit()
            .return_result_of(|| Ok(1500));
        link.set_get_max_transmission_unit(mtu);

        let recv = link.method_recv().set_result(Err(LinkError::Busy));
        link.set_recv(recv);

        let mut dev = new_test_dev(link);

        assert!(match dev.recv() {
            Err(Error::Link(LinkError::Busy)) => true,
            _ => false,
        });
    }

    #[test]
    fn test_recv_nothing() {
        let mut link = MockLink::new();

        let mtu = link.method_get_max_transmission_unit()
            .return_result_of(|| Ok(1500));
        link.set_get_max_transmission_unit(mtu);

        let recv = link.method_recv().set_result(Ok(0));
        link.set_recv(recv);

        let mut dev = new_test_dev(link);

        assert!(match dev.recv() {
            Err(Error::Nothing) => true,
            _ => false,
        });
    }
}
