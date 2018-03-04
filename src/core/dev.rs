use Result;
use core::layers::{
    EthernetAddress,
    Ipv4AddressCidr,
};

/// A low level interface for sending frames.
pub trait Device {
    /// Sends a frame across the link.
    fn send(&mut self, buffer: &[u8]) -> Result<()>;

    /// Reads a frame from the link and returns the size of frame.
    ///
    /// The buffer should be at least max_transmission_unit() bytes long to
    /// avoid errors.
    fn recv(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Returns the [MTU](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
    /// of the link.
    fn max_transmission_unit(&self) -> usize;

    /// Returns the Ipv4 address associated with the device.
    fn ipv4_addr(&self) -> Ipv4AddressCidr;

    /// Returns the ethernet address associated with the device.
    fn ethernet_addr(&self) -> EthernetAddress;
}
