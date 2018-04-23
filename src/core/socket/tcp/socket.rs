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

/// A TCP socket for reliable stream transfers created. Sockets can be created
/// by (1) opening client connections to a server or (2) dequeueing established
/// connections with accept(...).
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

    /// Dequeues zero or more packet enqueued for sending via function f.
    ///
    /// The socket may have several enqueued sockets if it is a listener for which
    /// we dequeue packets via function f. One packet per socket is dequeued until
    /// f returns an error.
    pub fn send_dequeue<F, R>(&mut self, mut f: F) -> Result<R>
    where
        F: FnMut(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        self.inner.send_dequeue(&mut f)
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

    /// Begins listening for incoming connections.
    ///
    /// # Panics
    ///
    /// Causes a panic if the connection is not in the closed state!
    pub fn listen(&mut self, syn_queue_len: usize, est_queue_len: usize) {
        self.inner = match self.inner {
            TcpState::Closed(ref mut closed) => {
                TcpState::Listen(closed.to_listen(syn_queue_len, est_queue_len))
            }
            _ => panic!("TcpSocket::listen(...) requires a closed socket!"),
        }
    }

    /// Dequeues an established connection if one has been established.
    ///
    /// # Panics
    ///
    /// Causes a panic if the connection is not in the listening state!
    pub fn accept(&mut self) -> Option<TcpSocket> {
        match self.inner {
            TcpState::Listen(ref mut listen) => listen.accept().map(|established| TcpSocket {
                inner: TcpState::Established(established),
            }),
            _ => panic!("TcpSocket::accept(...) requires a listening socket!"),
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
