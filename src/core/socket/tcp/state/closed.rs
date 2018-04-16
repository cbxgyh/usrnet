use std::time::Duration;

use rand;

use core::socket::SocketAddr;
use core::socket::tcp::state::{
    Tcp,
    TcpContext,
    TcpSynSent,
};
use core::time::Env as TimeEnv;

/// The TCP CLOSED state.
#[derive(Debug)]
pub struct TcpClosed<T: TimeEnv> {
    pub context: TcpContext<T>,
}

impl<T: TimeEnv> Tcp<T> for TcpClosed<T> {}

impl<T: TimeEnv> TcpClosed<T> {
    /// Transitions from CLOSED to SYN_SENT in an attempt to establish a
    /// connection with the specified endpoint.
    pub fn to_syn_sent(self, socket_addr: SocketAddr) -> TcpSynSent<T> {
        TcpSynSent {
            sent_syn_at: None,
            seq_num: rand::random::<u32>(),
            connecting_to: socket_addr,
            retransmit_timeout: Duration::from_millis(500),
            context: self.context,
        }
    }
}
