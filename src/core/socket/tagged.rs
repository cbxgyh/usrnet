use Result;
use core::socket::{
    Packet,
    RawSocket,
    Socket,
};

/// One of many types of sockets.
pub enum TaggedSocket<'a> {
    Raw(RawSocket<'a>),
}

impl<'a> Socket for TaggedSocket<'a> {
    fn send_forward<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        match *self {
            TaggedSocket::Raw(ref mut socket) => socket.send_forward(f),
        }
    }

    fn recv_forward(&mut self, packet: &Packet) -> Result<()> {
        match *self {
            TaggedSocket::Raw(ref mut socket) => socket.recv_forward(packet),
        }
    }
}

impl<'a> TaggedSocket<'a> {
    /// Tries to convert a tagged socket to a raw socket.
    pub fn as_raw_socket(&mut self) -> Option<&mut RawSocket<'a>> {
        match *self {
            TaggedSocket::Raw(ref mut socket) => Some(socket),
        }
    }
}
