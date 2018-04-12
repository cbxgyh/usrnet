use std::time::Duration;

use rand;

use core::socket::SocketAddr;
use core::time::Env;

use super::{
    Tcp,
    TcpContext,
    TcpSynSent,
};

/// The TCP CLOSED state.
#[derive(Debug)]
pub struct TcpClosed<'a, T: Env> {
    pub context: TcpContext<'a, T>,
}

impl<'a, T: Env> Tcp<'a, T> for TcpClosed<'a, T> {}

impl<'a, T: Env> TcpClosed<'a, T> {
    /// Transitions from CLOSED to SYN_SENT in an attempt to establish a
    /// connection with the specified endpoint.
    pub fn to_syn_sent(self, addr: SocketAddr) -> TcpSynSent<'a, T> {
        TcpSynSent {
            sent_syn_at: None,
            seq_num: rand::random::<u32>(),
            connecting_to: addr,
            retransmit_timeout: Duration::from_millis(500),
            context: self.context,
        }
    }
}
