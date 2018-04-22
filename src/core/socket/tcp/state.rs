use std::rc::Rc;

use {
    Error,
    Result,
};
use core::repr::{
    Ipv4Repr,
    TcpRepr,
};
use core::socket::{
    SocketAddrLease,
    TcpClosed,
    TcpEstablished,
    TcpSynSent,
};
use core::time::Env as TimeEnv;

/// A generic interface for implementing TCP state behavior and transitions.
pub trait Tcp {
    /// Dequeues a packet enqueued for sending via function f.
    ///
    /// The packet is only dequeued if f does not return an error. In addition,
    /// returns the next TCP state the socket should transition to.
    fn send_dequeue<F, R>(&mut self, _f: F) -> (Option<TcpState>, Result<R>)
    where
        F: FnOnce(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        (None, Err(Error::Exhausted))
    }

    /// Enqueues a packet for receiving.
    ///
    /// In addition, returns the next TCP state the socket should transition to.
    fn recv_enqueue(
        &mut self,
        _ipv4_repr: &Ipv4Repr,
        _tcp_repr: &TcpRepr,
        _payload: &[u8],
    ) -> (Option<TcpState>, Result<()>) {
        (None, Err(Error::NoOp))
    }
}

/// One of several TCP states.
#[derive(Debug)]
pub enum TcpState {
    Closed(TcpClosed),
    SynSent(TcpSynSent),
    Established(TcpEstablished),
}

impl Tcp for TcpState {
    fn send_dequeue<F, R>(&mut self, f: F) -> (Option<TcpState>, Result<R>)
    where
        F: FnOnce(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        match *self {
            TcpState::Closed(ref mut tcp) => tcp.send_dequeue(f),
            TcpState::SynSent(ref mut tcp) => tcp.send_dequeue(f),
            TcpState::Established(ref mut tcp) => tcp.send_dequeue(f),
        }
    }

    fn recv_enqueue(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        tcp_repr: &TcpRepr,
        payload: &[u8],
    ) -> (Option<TcpState>, Result<()>) {
        match *self {
            TcpState::Closed(ref mut tcp) => tcp.recv_enqueue(ipv4_repr, tcp_repr, payload),
            TcpState::SynSent(ref mut tcp) => tcp.recv_enqueue(ipv4_repr, tcp_repr, payload),
            TcpState::Established(ref mut tcp) => tcp.recv_enqueue(ipv4_repr, tcp_repr, payload),
        }
    }
}

/// Shared information across TCP states.
#[derive(Clone, Debug)]
pub struct TcpContext {
    // This is an Rc because we only release the binding once all sockets
    // are dropped. A situation with many sockets sharing a binding occurs
    // when a server accepts client connections.
    pub binding: Rc<SocketAddrLease>,
    pub interface_mtu: usize,
    pub time_env: Rc<TimeEnv>,
}
