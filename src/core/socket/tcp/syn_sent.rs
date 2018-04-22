use std::time::{
    Duration,
    Instant,
};

use {
    Error,
    Result,
};
use core::repr::{
    Ipv4Packet,
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
    fn send_dequeue<F, R>(&mut self, f: F) -> (Option<TcpState>, Result<R>)
    where
        F: FnOnce(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        let now = self.context.time_env.now_instant();

        let send_syn = match self.sent_syn_at {
            None => true,
            Some(instant) => (now - instant) >= self.retransmit_timeout,
        };

        if !send_syn {
            return (None, Err(Error::Exhausted));
        }

        let mut tcp_repr = TcpRepr {
            src_port: self.context.binding.port,
            dst_port: self.connecting_to.port,
            seq_num: self.seq_num,
            ack_num: 0,
            flags: [false; 9],
            window_size: 128, // TODO: Set this to the size of our receive buffer?
            urgent_pointer: 0,
            max_segment_size: Some(536),
        };

        tcp_repr.flags[TcpRepr::FLAG_SYN] = true;

        // It's important that max_segment_size is a Some(...) when calculating MSS to
        // acount for MSS TCP header option!
        let mss = (self.context.interface_mtu - Ipv4Packet::<&[u8]>::MIN_HEADER_LEN
            - tcp_repr.header_len()) as u16;
        tcp_repr.max_segment_size = Some(mss);

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
                debug!("TCP socket {:?} sent SYN during active open.", self);
                self.sent_syn_at = Some(now);
                self.retransmit_timeout *= 2;
                (None, Ok(res))
            }
            Err(err) => {
                debug!(
                    "TCP socket {:?} encountered {:?} when sending SYN during active open.",
                    self, err
                );
                (None, Err(err))
            }
        }
    }

    fn recv_enqueue(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        tcp_repr: &TcpRepr,
        _: &[u8],
    ) -> (Option<TcpState>, Result<()>) {
        // Check if the packet is destined and valid for this socket.
        if ipv4_repr.dst_addr != self.context.binding.addr
            || tcp_repr.dst_port != self.context.binding.port
            || ipv4_repr.src_addr != self.connecting_to.addr
            || tcp_repr.src_port != self.connecting_to.port
        {
            return (None, Err(Error::NoOp));
        } else if !tcp_repr.flags[TcpRepr::FLAG_ACK] || tcp_repr.ack_num != self.seq_num + 1 {
            // TODO: Handle simulatenous open which will not have an ACK flag, only SYN.
            return (None, Err(Error::NoOp));
        } else if tcp_repr.flags[TcpRepr::FLAG_RST] {
            debug!("TCP socket {:?} received RST, transition to CLOSED.", self);
            return (Some(TcpState::Closed(self.to_closed())), Ok(()));
        } else if tcp_repr.flags[TcpRepr::FLAG_SYN] {
            debug!(
                "TCP socket {:?} received SYN, transition to ESTABLISHED.",
                self
            );
            return (
                Some(TcpState::Established(self.to_established(tcp_repr.seq_num))),
                Ok(()),
            );
        }

        (None, Err(Error::NoOp))
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
    pub fn to_established(&mut self, ack_num: u32) -> TcpEstablished {
        TcpEstablished {
            connected_to: self.connecting_to,
            ack_num: ack_num + 1,
            ack_sent: false,
            seq_num: self.seq_num + 1,
            context: self.context.clone(),
        }
    }
}
