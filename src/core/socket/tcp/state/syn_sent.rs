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
    Packet,
    SocketAddr,
};
use core::time::Env;

use super::{
    Tcp,
    TcpContext,
    TcpState,
};

/// The TCP SYN_SENT state.
pub struct TcpSynSent<'a, T: Env> {
    pub sent_syn_to: SocketAddr,
    pub sent_syn_at: Option<Instant>,
    pub retransmit_timeout: Duration,
    pub seq_num: u32,
    pub context: TcpContext<'a, T>,
}

impl<'a, T: Env> Tcp<'a, T> for TcpSynSent<'a, T> {
    fn send_forward<F, R>(self, f: F) -> (TcpState<'a, T>, Result<R>)
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        let now = self.context.time_env.now_instant();

        let send_syn = match self.sent_syn_at {
            None => true,
            Some(instant) => (now - instant) >= self.retransmit_timeout,
        };

        if !send_syn {
            return (self.into(), Err(Error::Exhausted));
        }

        let mut tcp_repr = TcpRepr {
            src_port: self.context.binding.port,
            dst_port: self.sent_syn_to.port,
            seq_num: self.seq_num,
            ack_num: 0,
            flags: [false; 9],
            window_size: 128, // TODO: Initialize this to the size of our receive buffer?
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
            dst_addr: self.sent_syn_to.addr,
            protocol: Ipv4Protocol::TCP,
            payload_len: tcp_repr.header_len() as u16,
        };

        let mut payload = [0; 0];

        let packet = Packet::Tcp(ipv4_repr, tcp_repr, &mut payload[..]);

        // Caution, consider send failures! This can happen if the destination IP is
        // not in the ARP cache yet. In such a case, don't move forward the last
        // time we sent a SYN.
        match f(packet) {
            Ok(res) => {
                let syn_sent = TcpSynSent {
                    sent_syn_to: self.sent_syn_to,
                    sent_syn_at: Some(now),
                    retransmit_timeout: self.retransmit_timeout * 2,
                    seq_num: self.seq_num,
                    context: self.context,
                };
                (TcpState::from(syn_sent), Ok(res))
            }
            Err(err) => (self.into(), Err(err)),
        }
    }
}
