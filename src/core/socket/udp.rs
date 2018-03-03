use {
    Error,
    Result,
};
use core::layers::{
    Ipv4Protocol,
    Ipv4Repr,
    UdpPacket,
    UdpRepr,
};
use core::socket::{
    AddrLease,
    Packet,
    Socket,
    SocketAddr,
};
use core::storage::{
    Ring,
    Slice,
};

/// A UDP socket.
pub struct UdpSocket<'a> {
    binding: AddrLease<'a>,
    send_buffer: Ring<'a, (Slice<'a, u8>, SocketAddr)>,
    recv_buffer: Ring<'a, (Slice<'a, u8>, SocketAddr)>,
}

impl<'a> Socket for UdpSocket<'a> {
    fn send_forward<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        let binding = self.binding.clone();

        self.send_buffer
            .dequeue_maybe(|&mut (ref mut buffer, addr)| {
                let payload_len = buffer.len();

                let udp_repr = UdpRepr {
                    src_port: binding.port,
                    dst_port: addr.port,
                    length: UdpPacket::<&[u8]>::buffer_len(payload_len) as u16,
                };

                let ip_repr = Ipv4Repr {
                    src_addr: binding.addr,
                    dst_addr: addr.addr,
                    protocol: Ipv4Protocol::UDP,
                    payload_len: udp_repr.buffer_len() as u16,
                };

                let packet = Packet::Udp(ip_repr, udp_repr, &mut buffer[..]);
                f(packet)
            })
    }

    fn recv_forward(&mut self, packet: &Packet) -> Result<()> {
        let binding = self.binding.clone();

        self.recv_buffer
            .enqueue_maybe(|&mut (ref mut buffer, ref mut addr)| match *packet {
                Packet::Udp(ref ip_repr, ref udp_repr, ref payload) => {
                    if ip_repr.dst_addr != binding.addr || udp_repr.dst_port != binding.port {
                        Err(Error::NoOp)
                    } else {
                        buffer.try_resize(payload.len(), 0)?;
                        buffer.copy_from_slice(payload);
                        addr.addr = ip_repr.src_addr;
                        addr.port = udp_repr.src_port;
                        Ok(())
                    }
                }
                _ => Err(Error::NoOp),
            })
    }
}

impl<'a> UdpSocket<'a> {
    /// Creates a new UDP socket.
    pub fn new(
        binding: AddrLease<'a>,
        send_buffer: Ring<'a, (Slice<'a, u8>, SocketAddr)>,
        recv_buffer: Ring<'a, (Slice<'a, u8>, SocketAddr)>,
    ) -> UdpSocket<'a> {
        UdpSocket {
            binding: binding,
            send_buffer,
            recv_buffer,
        }
    }

    /// Enqueues a packet with buffer_len bytes for sending to the specified
    /// address.
    ///
    /// # Errors
    ///
    /// An Error::Exhausted occurs if the send buffer is full.
    pub fn send(&mut self, buffer_len: usize, addr: SocketAddr) -> Result<&mut [u8]> {
        self.send_buffer
            .enqueue_maybe(|&mut (ref mut buffer, ref mut addr_)| {
                buffer.try_resize(buffer_len, 0)?;

                for i in 0 .. buffer_len {
                    buffer[i] = 0;
                }

                *addr_ = addr;

                return Ok(&mut buffer[.. buffer_len]);
            })
    }

    /// Dequeues a received packet along with it's source address from the socket.
    ///
    /// # Errors
    ///
    /// An Error::Exhausted occurs if the receive buffer is full.
    pub fn recv(&mut self) -> Result<(&[u8], SocketAddr)> {
        self.recv_buffer
            .dequeue_with(|&mut (ref buffer, ref addr)| (&buffer[..], addr.clone()))
    }
}
