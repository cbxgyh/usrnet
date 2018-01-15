use std;

#[derive(Debug)]
pub enum Error {
    /// Indicates a generic IO error.
    IO(std::io::Error),
    /// Indicates the link is busy.
    Busy,
    /// Indicates a miscellaneous error with a message.
    Unknown(&'static str),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IO(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A low level interface for sending frames across a link.
pub trait Link {
    /// Sends a frame across the link.
    fn send(&mut self, buffer: &[u8]) -> Result<()>;

    /// Reads a frame from the link and returns the size of frame. You should
    /// ensure that the buffer has at least MTU bytes to avoid errors.
    fn recv(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Returns the [MTU](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
    /// of the link.
    fn get_max_transmission_unit(&self) -> Result<usize>;
}
