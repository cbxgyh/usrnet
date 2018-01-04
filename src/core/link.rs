use std;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A low level interface for sending frames across a link.
pub trait Link {
    /// Sends a frame across a link.
    fn send(&mut self, buf: &[u8]) -> Result<()>;

    /// Reads a frame from the underlying hardware and returns the size of
    /// frame. You should ensure that the buf has at least MTU bytes to avoid
    /// errors.
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Returns the [MTU](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
    /// of the underlying hardware.
    fn max_transmission_unit(&self) -> Result<usize>;
}
