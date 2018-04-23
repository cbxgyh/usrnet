use std::collections::VecDeque;
use std::time::Duration;

use rand;

use core::socket::{
    SocketAddr,
    Tcp,
    TcpContext,
    TcpListen,
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

    /// Transitions from CLOSED to LISTENING in order to accept connection
    /// requests.
    pub fn to_listen(&mut self, syn_queue_len: usize, est_queue_len: usize) -> TcpListen {
        TcpListen {
            syn_queue: VecDeque::with_capacity(syn_queue_len),
            est_queue: VecDeque::with_capacity(est_queue_len),
            context: self.context.clone(),
        }
    }
}
