use core::socket::{
    RawSocket,
    TcpSocket,
    UdpSocket,
};

/// One of many types of sockets.
pub enum TaggedSocket {
    Raw(RawSocket),
    Udp(UdpSocket),
    Tcp(TcpSocket),
}

impl TaggedSocket {
    /// Returns a reference to the underlying raw socket.
    ///
    /// # Panics
    ///
    /// Panics if the underlying socket is not a raw socket.
    pub fn as_raw_socket(&mut self) -> &mut RawSocket {
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
    pub fn as_tcp_socket(&mut self) -> &mut TcpSocket {
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
    pub fn as_udp_socket(&mut self) -> &mut UdpSocket {
        match *self {
            TaggedSocket::Udp(ref mut socket) => socket,
            _ => panic!("Not a UDP socket!"),
        }
    }
}
