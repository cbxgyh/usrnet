mod closed;
mod established;
mod syn_sent;

use {
    Error,
    Result,
};
use core::socket::{
    Packet,
    SocketAddrLease,
};
use core::time::Env as TimeEnv;

pub use self::closed::TcpClosed;
pub use self::established::TcpEstablished;
pub use self::syn_sent::TcpSynSent;

/// A generic interface for implementing TCP state behavior and transitions.
pub trait Tcp<T: TimeEnv>: Into<TcpState<T>> {
    /// Similar to Socket::send_forward(...). In addition to the result of f,
    /// the next state (or same but updated state) of the TCP FSM is returned.
    fn send_forward<F, R>(self, _: F) -> (TcpState<T>, Result<R>)
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        (self.into(), Err(Error::Exhausted))
    }

    /// Similar to Socket::recv_forward(...). In addition to a packet processing
    /// result, the next state (or same but updated state) of the TCP FSM is
    /// returned.
    fn recv_forward(self, _: &Packet) -> (TcpState<T>, Result<()>) {
        (self.into(), Err(Error::NoOp))
    }
}

/// One of several TCP states.
#[derive(Debug)]
pub enum TcpState<T: TimeEnv> {
    Closed(TcpClosed<T>),
    SynSent(TcpSynSent<T>),
    Established(TcpEstablished<T>),
}

impl<T: TimeEnv> From<TcpClosed<T>> for TcpState<T> {
    fn from(closed: TcpClosed<T>) -> TcpState<T> {
        TcpState::Closed(closed)
    }
}

impl<T: TimeEnv> From<TcpSynSent<T>> for TcpState<T> {
    fn from(syn_sent: TcpSynSent<T>) -> TcpState<T> {
        TcpState::SynSent(syn_sent)
    }
}

impl<T: TimeEnv> From<TcpEstablished<T>> for TcpState<T> {
    fn from(established: TcpEstablished<T>) -> TcpState<T> {
        TcpState::Established(established)
    }
}

impl<T: TimeEnv> Tcp<T> for TcpState<T> {
    fn send_forward<F, R>(self, f: F) -> (TcpState<T>, Result<R>)
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        match self {
            TcpState::Closed(closed) => closed.send_forward(f),
            TcpState::SynSent(syn_sent) => syn_sent.send_forward(f),
            TcpState::Established(established) => established.send_forward(f),
        }
    }

    fn recv_forward(self, packet: &Packet) -> (TcpState<T>, Result<()>) {
        match self {
            TcpState::Closed(closed) => closed.recv_forward(packet),
            TcpState::SynSent(syn_sent) => syn_sent.recv_forward(packet),
            TcpState::Established(established) => established.recv_forward(packet),
        }
    }
}

/// Shared information across TCP states.
#[derive(Debug)]
pub struct TcpContext<T>
where
    T: TimeEnv,
{
    pub binding: SocketAddrLease,
    pub interface_mtu: usize,
    pub time_env: T,
}
