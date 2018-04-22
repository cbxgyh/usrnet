use std::rc::Rc;

use Result;
use core::repr::{
    Ipv4Repr,
    TcpRepr,
};
use core::socket::{
    SocketAddr,
    SocketAddrLease,
    Tcp,
    TcpClosed,
    TcpContext,
    TcpState,
};
use core::time::Env as TimeEnv;

/// A TCP socket.
#[derive(Debug)]
pub struct TcpSocket {
    inner: TcpState,
}

impl TcpSocket {
    /// Creates a new TCP socket.
    pub fn new<T: 'static + TimeEnv>(
        binding: SocketAddrLease,
        interface_mtu: usize,
        time_env: T,
    ) -> TcpSocket {
        let context = TcpContext {
            binding: Rc::new(binding),
            interface_mtu,
            time_env: Rc::new(time_env),
        };
        let closed = TcpClosed { context };
        TcpSocket {
            inner: TcpState::Closed(closed),
        }
    }

    /// Dequeues a packet enqueued for sending via function f.
    ///
    /// The packet is only dequeued if f does not return an error.
    pub fn send_dequeue<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        let (tcp, ok_or_err) = self.inner.send_dequeue(f);
        if let Some(tcp) = tcp {
            self.inner = tcp;
        }
        ok_or_err
    }

    /// Enqueues a packet for receiving.
    pub fn recv_enqueue(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        tcp_repr: &TcpRepr,
        payload: &[u8],
    ) -> Result<()> {
        let (tcp, ok_or_err) = self.inner.recv_enqueue(ipv4_repr, tcp_repr, payload);
        if let Some(tcp) = tcp {
            self.inner = tcp;
        }
        ok_or_err
    }

    /// Initiates a connection to a TCP endpoint.
    ///
    /// # Panics
    ///
    /// Causes a panic if the connection is not in the closed state!
    pub fn connect(&mut self, socket_addr: SocketAddr) {
        self.inner = match self.inner {
            TcpState::Closed(ref mut closed) => TcpState::SynSent(closed.to_syn_sent(socket_addr)),
            _ => panic!("TcpSocket::connect(...) requires a closed socket!"),
        }
    }

    /// Checks if the socket is closed. The socket may be closed for reasons
    /// including an explicit close, timeout, reset, etc.
    pub fn is_closed(&self) -> bool {
        match self.inner {
            TcpState::Closed(_) => true,
            _ => false,
        }
    }

    /// Checks if the socket is connecting to an endpoint.
    pub fn is_establishing(&self) -> bool {
        match self.inner {
            TcpState::SynSent(_) => true,
            _ => false,
        }
    }

    /// Checks if the socket has connected to an endpoint.
    pub fn is_connected(&self) -> bool {
        match self.inner {
            TcpState::Established(_) => true,
            _ => false,
        }
    }
}
