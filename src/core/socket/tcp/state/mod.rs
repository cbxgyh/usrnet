mod closed;
mod syn_sent;

use {
    Error,
    Result,
};
use core::socket::{
    AddrLease,
    Packet,
};
use core::time::Env;

pub use self::closed::TcpClosed;
pub use self::syn_sent::TcpSynSent;

/// A generic interface for implementing TCP state behavior and transitions.
pub trait Tcp<'a, T: Env>: Into<TcpState<'a, T>> {
    /// Similar to Socket::send_forward(...). In addition to the result of f,
    /// the next state (or same but updated state) of the TCP FSM is returned.
    fn send_forward<F, R>(self, _: F) -> (TcpState<'a, T>, Result<R>)
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        (self.into(), Err(Error::Exhausted))
    }

    /// Similar to Socket::recv_forward(...). In addition to a packet processing
    /// result, the next state (or same but updated state) of the TCP FSM is
    /// returned.
    fn recv_forward(self, _: &Packet) -> (TcpState<'a, T>, Result<()>) {
        (self.into(), Err(Error::NoOp))
    }
}

/// One of several TCP states.
pub enum TcpState<'a, T: Env> {
    Closed(TcpClosed<'a, T>),
    SynSent(TcpSynSent<'a, T>),
}

impl<'a, T: Env> From<TcpClosed<'a, T>> for TcpState<'a, T> {
    fn from(closed: TcpClosed<'a, T>) -> TcpState<'a, T> {
        TcpState::Closed(closed)
    }
}

impl<'a, T: Env> From<TcpSynSent<'a, T>> for TcpState<'a, T> {
    fn from(syn_sent: TcpSynSent<'a, T>) -> TcpState<'a, T> {
        TcpState::SynSent(syn_sent)
    }
}

impl<'a, T: Env> Tcp<'a, T> for TcpState<'a, T> {
    fn send_forward<F, R>(self, f: F) -> (TcpState<'a, T>, Result<R>)
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        match self {
            TcpState::Closed(closed) => closed.send_forward(f),
            TcpState::SynSent(syn_sent) => syn_sent.send_forward(f),
        }
    }

    fn recv_forward(self, packet: &Packet) -> (TcpState<'a, T>, Result<()>) {
        match self {
            TcpState::Closed(closed) => closed.recv_forward(packet),
            TcpState::SynSent(syn_sent) => syn_sent.recv_forward(packet),
        }
    }
}

/// Shared information across TCP states.
pub struct TcpContext<'a, T>
where
    T: Env,
{
    pub binding: AddrLease<'a>,
    pub time_env: T,
    pub interface_mtu: usize,
}
