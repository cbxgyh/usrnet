use core::repr::{
    Ipv4Protocol,
    Ipv4Repr,
    TcpRepr,
};
use core::socket::{
    SocketAddr,
    Tcp,
    TcpContext,
};
use {
    Error,
    Result,
};

/// The TCP ESTABLISHED state.
#[derive(Debug)]
pub struct TcpEstablished {
    pub connected_to: SocketAddr,
    pub ack_num: u32,
    pub ack_sent: bool,
    pub seq_num: u32,
    pub context: TcpContext,
}

impl Tcp for TcpEstablished {
    fn send_dequeue<F, R>(&mut self, f: &mut F) -> Result<R>
    where
        F: FnMut(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        if self.ack_sent {
            return Err(Error::Exhausted);
        }

        // Send one ACK for now, retransmissions will be implemented later.
        let mut tcp_repr = TcpRepr {
            src_port: self.context.binding.port,
            dst_port: self.connected_to.port,
            seq_num: self.seq_num,
            ack_num: self.ack_num,
            flags: [false; 9],
            window_size: 128,
            urgent_pointer: 0,
            max_segment_size: None,
        };

        tcp_repr.flags[TcpRepr::FLAG_ACK] = true;

        let ipv4_repr = Ipv4Repr {
            src_addr: self.context.binding.addr,
            dst_addr: self.connected_to.addr,
            protocol: Ipv4Protocol::TCP,
            payload_len: tcp_repr.header_len() as u16,
        };

        match f(&ipv4_repr, &tcp_repr, &[0; 0]) {
            Ok(res) => {
                debug!(
                    "ESTABLISHED @ ({}, {}) sent ACK for SEQ_NUM {}.",
                    self.context.binding, self.connected_to, self.ack_num
                );
                self.ack_sent = true;
                Ok(res)
            }
            Err(err) => {
                debug!(
                    "ESTABLISHED @ ({}, {}) encountered {:?} when sending ACK for SEQ_NUM {}.",
                    self.context.binding, self.connected_to, err, self.ack_num
                );
                Err(err)
            }
        }
    }
}

impl TcpEstablished {
    /// Checks if the state accepts packets with particular (source,
    /// destination) addresses.
    pub fn accepts(&self, src_addr: &SocketAddr, dst_addr: &SocketAddr) -> bool {
        (&self.connected_to == src_addr) && (self.context.binding.as_ref() == dst_addr)
    }
}
