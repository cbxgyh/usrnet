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
    Packet,
    SocketAddr,
};
use core::socket::tcp::state::{
    Tcp,
    TcpContext,
    TcpState,
};
use core::time::Env as TimeEnv;

/// The TCP ESTABLISHED state.
#[derive(Debug)]
pub struct TcpEstablished<T: TimeEnv> {
    pub connected_to: SocketAddr,
    pub ack_num: u32,
    pub ack_sent: bool,
    pub seq_num: u32,
    pub context: TcpContext<T>,
}

impl<T: TimeEnv> Tcp<T> for TcpEstablished<T> {
    fn send_forward<F, R>(self, f: F) -> (TcpState<T>, Result<R>)
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        if self.ack_sent {
            return (self.into(), Err(Error::Exhausted));
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

        let mut payload = [0; 0];
        let packet = Packet::Tcp((ipv4_repr, tcp_repr, &mut payload[..]));

        match f(packet) {
            Ok(res) => {
                debug!(
                    "TCP socket {:?} sent ACK for SEQ_NUM {:?}.",
                    self, self.ack_num
                );
                let established = TcpEstablished {
                    connected_to: self.connected_to,
                    ack_num: self.ack_num,
                    ack_sent: true,
                    seq_num: self.seq_num,
                    context: self.context,
                };
                (TcpState::from(established), Ok(res))
            }
            Err(err) => {
                debug!(
                    "TCP socket {:?} encountered {:?} when sending ACK for SEQ_NUM {:?}.",
                    self, err, self.ack_num
                );
                (self.into(), Err(err))
            }
        }
    }
}
