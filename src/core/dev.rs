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
    Buffer,
    /// Indicates a situation with an empty link.
    Nothing,
}

impl From<LinkError> for Error {
    fn from(err: LinkError) -> Self {
        Error::Link(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// High level interface for sending frames across a link.
pub trait Device {
    /// Sends a frame via the underlying link.
    ///
    /// F receives a writable reference to the send buffer which will be
    /// sent via the underlying link.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the send request or
    /// underlying link.
    fn send<F, R>(&mut self, buffer_len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> R;

    /// Receives a frame via the underlying link.
    ///
    /// F receives a reference to the receive buffer which it can process
    /// as desired.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the underlying link,
    /// including a lack of data.
    fn recv<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R;

    /// Returns the Ipv4 address associated with the device.
    fn get_ipv4_addr(&self) -> Ipv4Address;

    /// Returns the ethernet address associated with the device.
    fn get_ethernet_addr(&self) -> EthernetAddress;
}

/// Device which reuses preallocated send/recv buffers.
pub struct Standard<T, U>
where
    T: Link,
    U: AsRef<[u8]> + AsMut<[u8]>,
{
    link: T,
    buffer: U,
    ipv4_addr: Ipv4Address,
    eth_addr: EthernetAddress,
    max_transmission_unit: usize,
}

impl<T, U> Standard<T, U>
where
    T: Link,
    U: AsRef<[u8]> + AsMut<[u8]>,
{
    /// Creates a Standard device.
    ///
    /// # Errors
    ///
    /// Causes an error if the buffer length is not at least twice the link
    /// MTU or the link is down.
    pub fn try_new(
        link: T,
        buffer: U,
        ipv4_addr: Ipv4Address,
        eth_addr: EthernetAddress,
    ) -> Result<Standard<T, U>> {
        let max_transmission_unit = link.get_max_transmission_unit()?;

        if buffer.as_ref().len() < max_transmission_unit * 2 {
            return Err(Error::Buffer);
        }

        Ok(Standard {
            link,
            buffer,
            ipv4_addr,
            eth_addr,
            max_transmission_unit,
        })
    }
}

impl<T, U> Device for Standard<T, U>
where
    T: Link,
    U: AsRef<[u8]> + AsMut<[u8]>,
{
    fn send<F, R>(&mut self, buffer_len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        if buffer_len > self.max_transmission_unit {
            return Err(Error::Buffer);
        }

        for i in 0..buffer_len {
            self.buffer.as_mut()[i] = 0;
        }

        let send_buffer = &mut self.buffer.as_mut()[..buffer_len];

        let res = f(send_buffer);

        self.link.send(send_buffer)?;

        Ok(res)
    }

    fn recv<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> R,
    {
        let recv_buffer = &mut self.buffer.as_mut()[self.max_transmission_unit..];

        match self.link.recv(recv_buffer) {
            Ok(buffer_len) => {
                if buffer_len == 0 {
                    Err(Error::Nothing)
                } else {
                    Ok(f(&recv_buffer[..buffer_len]))
                }
            }
            Err(err) => Err(Error::Link(err)),
        }
    }

    fn get_ipv4_addr(&self) -> Ipv4Address {
        self.ipv4_addr
    }

    fn get_ethernet_addr(&self) -> EthernetAddress {
        self.eth_addr
    }
}

#[cfg(test)]
mod tests {
    use core::link::MockLink;
    use super::*;

    fn new_test_dev(link: MockLink) -> Standard<MockLink, std::vec::Vec<u8>> {
        Standard::try_new(
            link,
            vec![0; 10240],
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

        assert_eq!(
            dev.send(1, |buffer| {
                buffer[0] = 9;
                9
            }).unwrap(),
            9
        );

        assert_eq!(dev.buffer[0], 9);

        assert_eq!(
            dev.send(1, |buffer| {
                assert_eq!(buffer[0], 0);
                10
            }).unwrap(),
            10
        );
    }

    #[test]
    fn test_send_overflow() {
        let mut link = MockLink::new();

        let mtu = link.method_get_max_transmission_unit()
            .return_result_of(|| Ok(100));
        link.set_get_max_transmission_unit(mtu);

        let mut dev = new_test_dev(link);

        assert_matches!(dev.send(101, |_| {}), Err(Error::Buffer));
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

        assert_eq!(
            dev.recv(|buffer| {
                assert_eq!(buffer.len(), 100);
                100
            }).unwrap(),
            100
        );
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

        assert_matches!(dev.recv(|_| {}), Err(Error::Link(LinkError::Busy)));
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

        assert_matches!(dev.recv(|_| {}), Err(Error::Nothing));
    }
}
