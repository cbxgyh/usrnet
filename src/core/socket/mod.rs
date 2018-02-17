pub mod raw;

pub use self::raw::RawSocket;

/// One of many types of sockets.
pub enum Socket<'a> {
    RawSocket(RawSocket<'a>),
}

impl<'a> Socket<'a> {
    /// Attempts performing a temporary conversion to a raw socket.
    pub fn try_as_raw_socket(&mut self) -> Option<&mut RawSocket<'a>> {
        match *self {
            Socket::RawSocket(ref mut raw_socket) => Some(raw_socket),
        }
    }
}
