use std::time::{
    Duration,
    Instant,
};

use {
    Error,
    Result,
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

/// The TCP SYN_RECV state.
#[derive(Debug)]
pub struct TcpSynRecv {
    pub connecting_to: SocketAddr,
    pub sent_syn_ack_at: Option<Instant>,
    pub seq_num: u32,
    pub ack_num: u32,
    pub retransmit_timeout: Duration,
    pub context: TcpContext,
}

impl Tcp for TcpSynRecv {
    fn send_dequeue<F, R>(&mut self, f: &mut F) -> Result<R>
    where
        F: FnMut(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        let now = self.context.time_env.now_instant();

        let send_syn = match self.sent_syn_ack_at {
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
            ack_num: self.ack_num,
            flags: [false; 9],
            // TODO: Set this to the size of our receive buffer?
            window_size: 128,
            urgent_pointer: 0,
            // TODO: Path MTU discovery to determine MSS.
            max_segment_size: Some(536),
        };

        tcp_repr.flags[TcpRepr::FLAG_ACK] = true;
        tcp_repr.flags[TcpRepr::FLAG_SYN] = true;

        let ipv4_repr = Ipv4Repr {
            src_addr: self.context.binding.addr,
            dst_addr: self.connecting_to.addr,
            protocol: Ipv4Protocol::TCP,
            payload_len: tcp_repr.header_len() as u16,
        };

        match f(&ipv4_repr, &tcp_repr, &[0; 0]) {
            Ok(res) => {
                debug!(
                    "SYN_RECV @ ({}, {}) sent SYN + ACK.",
                    self.context.binding, self.connecting_to
                );
                self.sent_syn_ack_at = Some(now);
                self.retransmit_timeout *= 2;
                Ok(res)
            }
            Err(err) => {
                debug!(
                    "SYN_RECV @ ({}, {}) encountered {:?} when sending SYN during active open.",
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
                "SYN_RECV @ ({}, {}) received RST, transition to CLOSED.",
                self.context.binding, self.connecting_to
            );
            return (Some(TcpState::Closed(self.to_closed())), Ok(()));
        }

        if tcp_repr.flags[TcpRepr::FLAG_ACK] {
            debug!(
                "SYN_RECV @ ({}, {}) received ACK, transition to ESTABLISHED.",
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

impl TcpSynRecv {
    /// Checks if the state accepts packets with particular (source, destination)
    /// addresses.
    pub fn accepts(&self, src_addr: &SocketAddr, dst_addr: &SocketAddr) -> bool {
        (&self.connecting_to == src_addr) && (self.context.binding.as_ref() == dst_addr)
    }

    /// Transitions from SYN_RECV to CLOSED in response to a RST + ACK.
    pub fn to_closed(&mut self) -> TcpClosed {
        TcpClosed {
            context: self.context.clone(),
        }
    }

    /// Transitions from SYN_RECV to ESTABLISHED in response to a SYN + ACK.
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
