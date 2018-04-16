pub mod state;

use Result;
use core::socket::{
    Packet,
    Socket,
    SocketAddr,
    SocketAddrLease,
};
use core::time::{
    Env as TimeEnv,
    SystemEnv,
};

use self::state::{
    Tcp,
    TcpClosed,
    TcpContext,
    TcpState,
};

/// A TCP socket.
#[derive(Debug)]
pub struct TcpSocket<T: TimeEnv = SystemEnv> {
    // Use an Option to implement consumable states behind a mutable socket
    // abstraction.
    inner: Option<TcpState<T>>,
}

impl<T: TimeEnv> Socket for TcpSocket<T> {
    fn send_forward<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        let inner = self.inner.take().unwrap();
        let (inner, res) = inner.send_forward(f);
        self.inner = Some(inner);
        res
    }

    fn recv_forward(&mut self, packet: &Packet) -> Result<()> {
        let inner = self.inner.take().unwrap();
        let (inner, res) = inner.recv_forward(packet);
        self.inner = Some(inner);
        res
    }
}

impl<T: TimeEnv> TcpSocket<T> {
    /// Creates a new TCP socket.
    pub fn new(binding: SocketAddrLease, time_env: T, interface_mtu: usize) -> TcpSocket<T> {
        let context = TcpContext {
            binding,
            time_env,
            interface_mtu,
        };
        let closed = TcpClosed { context };
        let inner = TcpState::from(closed);
        TcpSocket { inner: Some(inner) }
    }

    /// Initiates a connection to a TCP endpoint.
    ///
    /// # Panics
    ///
    /// Causes a panic if the connection is not in the closed state!
    pub fn connect(&mut self, socket_addr: SocketAddr) {
        match self.inner.take() {
            Some(TcpState::Closed(closed)) => {
                self.inner = Some(TcpState::from(closed.to_syn_sent(socket_addr)))
            }
            _ => panic!("TcpSocket::connect(...) requires a closed socket!"),
        }
    }

    /// Checks if the socket is closed. The socket may be closed for reasons
    /// including an explicit close, timeout, reset, etc.
    pub fn is_closed(&self) -> bool {
        match self.inner {
            Some(TcpState::Closed(_)) => true,
            _ => false,
        }
    }

    /// Checks if the socket is connecting to an endpoint.
    pub fn is_establishing(&self) -> bool {
        match self.inner {
            Some(TcpState::SynSent(_)) => true,
            _ => false,
        }
    }

    /// Checks if the socket has connected to an endpoint.
    pub fn is_connected(&self) -> bool {
        match self.inner {
            Some(TcpState::Established(_)) => true,
            _ => false,
        }
    }
}
