use std::collections::VecDeque;
use std::time::Duration;

use rand;

use core::repr::{
    Ipv4Repr,
    TcpRepr,
};
use core::socket::{
    SocketAddr,
    Tcp,
    TcpContext,
    TcpEstablished,
    TcpState,
    TcpSynRecv,
};
use {
    Error,
    Result,
};

/// The TCP LISTENING state.
#[derive(Debug)]
pub struct TcpListen {
    pub syn_queue: VecDeque<TcpSynRecv>,
    pub est_queue: VecDeque<TcpEstablished>,
    pub context: TcpContext,
}

impl Tcp for TcpListen {
    #[allow(unused_must_use)]
    fn send_dequeue<F, R>(&mut self, f: &mut F) -> Result<R>
    where
        F: FnMut(&Ipv4Repr, &TcpRepr, &[u8]) -> Result<R>,
    {
        // We don't have anything to send, but any enqueued states might.
        for syn_recv in self.syn_queue.iter_mut() {
            // TODO: Handle SYN_RECV timeouts. Common DDoS tactic.
            syn_recv.send_dequeue(f);
        }

        for est in self.est_queue.iter_mut() {
            est.send_dequeue(f);
        }

        Err(Error::Exhausted)
    }

    fn recv_enqueue(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        tcp_repr: &TcpRepr,
        payload: &[u8],
    ) -> (Option<TcpState>, Result<()>) {
        // Check if the packet is destined for this socket (or any children).
        if ipv4_repr.dst_addr != self.context.binding.addr
            || tcp_repr.dst_port != self.context.binding.port
        {
            return (None, Err(Error::Ignored));
        }

        // Forward the packet to any establishing/established connections first.
        if self.recv_enqueue_syn(ipv4_repr, tcp_repr, payload) {
            debug!(
                "LISTEN @ {} ignoring packet, accepted by SYN_RECV socket.",
                self.context.binding
            );
            return (None, Ok(()));
        }

        if self.recv_enqueue_est(ipv4_repr, tcp_repr, payload) {
            debug!(
                "LISTEN @ {} ignoring packet, accepted by ESTABLISHED socket.",
                self.context.binding
            );
            return (None, Ok(()));
        }

        // See if we can establish a new connection. None of the existing sockets want
        // to accept the packet so this is our only option left.
        if !tcp_repr.flags[TcpRepr::FLAG_SYN] || tcp_repr.flags[TcpRepr::FLAG_ACK]
            || tcp_repr.flags[TcpRepr::FLAG_RST]
        {
            // Check if the packet is a valid SYN.
            return (None, Err(Error::Ignored));
        }

        if self.syn_queue.capacity() == self.syn_queue.len() {
            // Check if we have space on our SYN queue.
            debug!(
                "LISTEN @ {} ignoring packet, no capacity in SYN queue.",
                self.context.binding
            );
            return (None, Err(Error::Exhausted));
        }

        let connecting_to = SocketAddr {
            addr: ipv4_repr.src_addr,
            port: tcp_repr.src_port,
        };
        let syn_recv = self.to_syn_recv(connecting_to, tcp_repr.seq_num);
        debug!(
            "LISTEN @ {} enqueueing SYN_RECV socket with connection to {}.",
            self.context.binding, connecting_to
        );
        self.syn_queue.push_back(syn_recv);
        (None, Ok(()))
    }
}

impl TcpListen {
    /// Dequeues an established connection if one exists.
    pub fn accept(&mut self) -> Option<TcpEstablished> {
        self.est_queue.pop_front()
    }

    /// Forwards a packet to an ESTABLISHED state.
    ///
    /// Returns a boolean indicating if the packet was acceptable by any
    /// sockets.
    pub fn recv_enqueue_syn(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        tcp_repr: &TcpRepr,
        payload: &[u8],
    ) -> bool {
        let src_addr = SocketAddr {
            addr: ipv4_repr.src_addr,
            port: tcp_repr.src_port,
        };
        let dst_addr = SocketAddr {
            addr: ipv4_repr.dst_addr,
            port: tcp_repr.dst_port,
        };

        for i in 0 .. self.syn_queue.len() {
            if !self.syn_queue[i].accepts(&src_addr, &dst_addr) {
                continue;
            }

            match self.syn_queue[i].recv_enqueue(ipv4_repr, tcp_repr, payload) {
                (None, Ok(())) => {
                    // Not an interesting event, don't log. (Might flood the log as well)
                }
                (Some(TcpState::Established(est)), _) => {
                    if self.est_queue.capacity() == self.est_queue.len() {
                        warn!(
                            "ESTABLISHED queue of LISTEN @ {} does not have \
                             capacity for another connection.",
                            self.context.binding
                        );
                    } else {
                        debug!(
                            "Moving SYN_RECV @ ({}, {}) to ESTABLISHED.",
                            self.syn_queue[i].context.binding, self.syn_queue[i].connecting_to
                        );
                        self.syn_queue.remove(i);
                        self.est_queue.push_back(est);
                    }
                }
                (Some(tcp), _) => {
                    // TODO: Handle special transitions like closes which lead to timed waits.
                    debug!(
                        "SYN_RECV @ ({}, {}) is transitioning to {}, dropping.",
                        self.syn_queue[i].context.binding,
                        self.syn_queue[i].connecting_to,
                        tcp.as_str()
                    );
                    self.syn_queue.remove(i);
                }
                (None, _) => {}
            };

            return true;
        }

        false
    }

    /// Forwards a packet to a SYN_RECV state.
    ///
    /// Returns a boolean indicating if the packet was acceptable by any
    /// sockets.
    pub fn recv_enqueue_est(
        &mut self,
        ipv4_repr: &Ipv4Repr,
        tcp_repr: &TcpRepr,
        payload: &[u8],
    ) -> bool {
        let src_addr = SocketAddr {
            addr: ipv4_repr.src_addr,
            port: tcp_repr.src_port,
        };
        let dst_addr = SocketAddr {
            addr: ipv4_repr.dst_addr,
            port: tcp_repr.dst_port,
        };

        for i in 0 .. self.est_queue.len() {
            if !self.est_queue[i].accepts(&src_addr, &dst_addr) {
                continue;
            }

            match self.est_queue[i].recv_enqueue(ipv4_repr, tcp_repr, payload) {
                (None, Ok(())) => {
                    // Not an interesting event, don't log. (Might flood the log as well)
                }
                (Some(tcp), _) => {
                    // TODO: Handle special transitions like closes which lead to timed waits.
                    debug!(
                        "ESTABLISHED @ ({}, {}) is transitioning to {}, dropping.",
                        self.est_queue[i].context.binding,
                        self.est_queue[i].connected_to,
                        tcp.as_str()
                    );
                    self.est_queue.remove(i);
                }
                (None, _) => {}
            };

            return true;
        }

        false
    }

    /// Transitions from LISTEN to SYN_RECV in order to establish a new
    /// connection.
    pub fn to_syn_recv(&mut self, connecting_to: SocketAddr, remote_seq_num: u32) -> TcpSynRecv {
        TcpSynRecv {
            sent_syn_ack_at: None,
            seq_num: rand::random::<u32>(),
            ack_num: remote_seq_num + 1,
            connecting_to,
            retransmit_timeout: Duration::from_secs(1),
            context: self.context.clone(),
        }
    }
}
