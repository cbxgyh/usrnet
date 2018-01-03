use std;

use core::storage::{Buffer, BufferMut};

#[derive(Debug)]
pub enum Error {
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Link {
    /// Writes a link frame into the buffer or returns an error.
    fn recv(buffer: &mut BufferMut) -> Result<()>;

    /// Writes a frame buffer onto the link or returns an error.
    fn send(buffer: &Buffer) -> Result<()>;
}
