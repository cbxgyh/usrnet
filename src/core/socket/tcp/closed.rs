use std::time::Duration;

use rand;

use core::socket::{
    SocketAddr,
    Tcp,
    TcpContext,
    TcpSynSent,
};

/// The TCP CLOSED state.
#[derive(Debug)]
pub struct TcpClosed {
    pub context: TcpContext,
}

impl Tcp for TcpClosed {}

impl TcpClosed {
    /// Transitions from CLOSED to SYN_SENT in an attempt to establish a
    /// connection with the specified endpoint.
    pub fn to_syn_sent(&mut self, socket_addr: SocketAddr) -> TcpSynSent {
        TcpSynSent {
            sent_syn_at: None,
            seq_num: rand::random::<u32>(),
            connecting_to: socket_addr,
            retransmit_timeout: Duration::from_secs(1),
            context: self.context.clone(),
        }
    }
}
