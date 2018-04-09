use Result;
use core::socket::{
    Packet,
    RawSocket,
    Socket,
    TcpSocket,
    UdpSocket,
};

/// One of many types of sockets.
pub enum TaggedSocket<'a> {
    Raw(RawSocket<'a>),
    Udp(UdpSocket<'a>),
    Tcp(TcpSocket<'a>),
    #[doc(hidden)] ___Exhaustive,
}

impl<'a> Socket for TaggedSocket<'a> {
    fn send_forward<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        match *self {
            TaggedSocket::Raw(ref mut socket) => socket.send_forward(f),
            TaggedSocket::Tcp(ref mut socket) => socket.send_forward(f),
            TaggedSocket::Udp(ref mut socket) => socket.send_forward(f),
            _ => panic!("Unsupported socket!"),
        }
    }

    fn recv_forward(&mut self, packet: &Packet) -> Result<()> {
        match *self {
            TaggedSocket::Raw(ref mut socket) => socket.recv_forward(packet),
            TaggedSocket::Tcp(ref mut socket) => socket.recv_forward(packet),
            TaggedSocket::Udp(ref mut socket) => socket.recv_forward(packet),
            _ => panic!("Unsupported socket!"),
        }
    }
}

impl<'a> TaggedSocket<'a> {
    /// Returns a reference to the underlying raw socket.
    ///
    /// # Panics
    ///
    /// Panics if the underlying socket is not a raw socket.
    pub fn as_raw_socket(&mut self) -> &mut RawSocket<'a> {
        match *self {
            TaggedSocket::Raw(ref mut socket) => socket,
            _ => panic!("Not a raw socket!"),
        }
    }

    /// Returns a reference to the underlying TCP socket.
    ///
    /// # Panics
    ///
    /// Panics if the underlying socket is not a TCP socket.
    pub fn as_tcp_socket(&mut self) -> &mut TcpSocket<'a> {
        match *self {
            TaggedSocket::Tcp(ref mut socket) => socket,
            _ => panic!("Not a TCP socket!"),
        }
    }

    /// Returns a reference to the underlying UDP socket.
    ///
    /// # Panics
    ///
    /// Panics if the underlying socket is not a UDP socket.
    pub fn as_udp_socket(&mut self) -> &mut UdpSocket<'a> {
        match *self {
            TaggedSocket::Udp(ref mut socket) => socket,
            _ => panic!("Not a UDP socket!"),
        }
    }
}
