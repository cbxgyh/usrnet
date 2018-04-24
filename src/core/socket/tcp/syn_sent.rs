use std::time::{
    Duration,
    Instant,
};

use core::repr::{
    Ipv4Protocol,
    Ipv4Repr,
    TcpRepr,
};
use core::socket::{
    SocketAddr,
    Tcp,
    TcpClosed,
    TcpContext,
    TcpEstablished,
    TcpState,
};
use {
    Error,
    Result,
};

/// The TCP SYN_SENT state.
#[derive(Debug)]
pub struct TcpSynSent {
    pub connecting_to: SocketAddr,
    pub sent_syn_at: Option<Instant>,
    pub seq_num: u32,
    pub retransmit_timeout: Duration,
    pub context: TcpContext,
}

impl Tcp for TcpSynSent {
    fn send_dequeue<F, R>(&mut self, f: &mut F) -> Result<R>
    where
        F: FnMut(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        let now = self.context.time_env.now_instant();

        let send_syn = match self.sent_syn_at {
            None => true,
            Some(instant) => (now - instant) >= self.retransmit_timeout,
        };

        if !send_syn {
            return Err(Error::Exhausted);
        }

        let mut tcp_repr = TcpRepr {
            src_port: self.context.binding.port,
            dst_port: self.connecting_to.port,
            seq_num: self.seq_num,
            ack_num: 0,
            flags: [false; 9],
            // TODO: Set this to the size of our receive buffer?
            window_size: 128,
            urgent_pointer: 0,
            // TODO: Path MTU discovery to determine MSS.
            max_segment_size: Some(536),
        };

        tcp_repr.flags[TcpRepr::FLAG_SYN] = true;

        let ipv4_repr = Ipv4Repr {
            src_addr: self.context.binding.addr,
            dst_addr: self.connecting_to.addr,
            protocol: Ipv4Protocol::TCP,
            payload_len: tcp_repr.header_len() as u16,
        };

        // Caution, consider send failures! This can happen if the destination IP is
        // not in the ARP cache yet. In such a case, don't move forward the last
        // time we sent a SYN.
        match f(&ipv4_repr, &tcp_repr, &[0; 0]) {
            Ok(res) => {
                debug!(
                    "SYN_SENT @ ({}, {}) sent SYN during active open.",
                    self.context.binding, self.connecting_to
                );
                self.sent_syn_at = Some(now);
                self.retransmit_timeout *= 2;
                Ok(res)
            }
            Err(err) => {
                debug!(
                    "SYN_SENT @ ({}, {}) encountered {:?} when sending SYN during active open.",
                    self.context.binding, self.connecting_to, err
                );
                Err(err)
            }
        }
    }

    fn recv_enqueue(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        tcp_repr: &TcpRepr,
        _: &[u8],
    ) -> (Option<TcpState>, Result<()>) {
        if ipv4_repr.dst_addr != self.context.binding.addr
            || tcp_repr.dst_port != self.context.binding.port
            || ipv4_repr.src_addr != self.connecting_to.addr
            || tcp_repr.src_port != self.connecting_to.port
            || tcp_repr.ack_num != self.seq_num + 1
        {
            return (None, Err(Error::Ignored));
        }

        if tcp_repr.flags[TcpRepr::FLAG_RST] {
            debug!(
                "SYN_SENT @ ({}, {}) received RST, transition to CLOSED.",
                self.context.binding, self.connecting_to
            );
            return (Some(TcpState::Closed(self.to_closed())), Ok(()));
        }

        if !tcp_repr.flags[TcpRepr::FLAG_ACK] {
            return (None, Err(Error::Ignored));
        }

        if tcp_repr.flags[TcpRepr::FLAG_SYN] {
            debug!(
                "SYN_SENT @ ({}, {}) received SYN, transition to ESTABLISHED.",
                self.context.binding, self.connecting_to
            );
            return (
                Some(TcpState::Established(self.to_established(tcp_repr.seq_num))),
                Ok(()),
            );
        }

        (None, Err(Error::Ignored))
    }
}

impl TcpSynSent {
    /// Transitions from SYN_SENT to CLOSED in response to a RST + ACK.
    pub fn to_closed(&mut self) -> TcpClosed {
        TcpClosed {
            context: self.context.clone(),
        }
    }

    /// Transitions from SYN_SENT to ESTABLISHED in response to a SYN + ACK.
    pub fn to_established(&mut self, remote_seq_num: u32) -> TcpEstablished {
        TcpEstablished {
            connected_to: self.connecting_to,
            ack_num: remote_seq_num + 1,
            ack_sent: false,
            seq_num: self.seq_num + 1,
            context: self.context.clone(),
        }
    }
}
