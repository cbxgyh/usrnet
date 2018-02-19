use {
    Error,
    Result,
};
use core::socket::{
    Packet,
    Socket,
};
use core::storage::{
    Ring,
    Slice,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawType {
    Ethernet,
    Ipv4,
}

/// Socket for sending and receiving raw ethernet or IP packets.
#[derive(Debug)]
pub struct RawSocket<'a> {
    send_buffer: Ring<'a, Slice<'a, u8>>,
    recv_buffer: Ring<'a, Slice<'a, u8>>,
    raw_type: RawType,
}

impl<'a> RawSocket<'a> {
    /// Creates a socket with the provided send and receive buffers.
    pub fn new(
        send_buffer: Ring<'a, Slice<'a, u8>>,
        recv_buffer: Ring<'a, Slice<'a, u8>>,
        raw_type: RawType,
    ) -> RawSocket<'a> {
        RawSocket {
            send_buffer,
            recv_buffer,
            raw_type,
        }
    }

    /// Enqueues a packet with buffer_len bytes for sending.
    ///
    /// # Errors
    ///
    /// An Error::Exhausted occurs if the send buffer is full.
    pub fn send(&mut self, buffer_len: usize) -> Result<&mut [u8]> {
        self.send_buffer.enqueue_maybe(|buffer| {
            buffer.try_resize(buffer_len, 0)?;

            for i in 0..buffer_len {
                buffer[i] = 0;
            }

            return Ok(&mut buffer[..buffer_len]);
        })
    }

    /// Dequeues a received packet from the socket.
    ///
    /// # Errors
    ///
    /// An Error::Exhausted occurs if the receive buffer is full.
    pub fn recv(&mut self) -> Result<&[u8]> {
        self.recv_buffer.dequeue_with(|buffer| &buffer[..])
    }
}

impl<'a> Socket for RawSocket<'a> {
    fn send_forward<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Packet) -> Result<R>,
    {
        let raw_type = self.raw_type;

        self.send_buffer.dequeue_maybe(|buffer| match raw_type {
            RawType::Ethernet => {
                let packet = Packet::Raw(&mut buffer[..]);
                f(packet)
            }
            RawType::Ipv4 => {
                let packet = Packet::Ipv4(&mut buffer[..]);
                f(packet)
            }
        })
    }

    fn recv_forward(&mut self, packet: &Packet) -> Result<()> {
        let raw_type = self.raw_type;

        self.recv_buffer.enqueue_maybe(|buffer| match *packet {
            Packet::Raw(ref eth_buffer) => {
                if raw_type != RawType::Ethernet {
                    return Err(Error::NoOp);
                }

                buffer.try_resize(eth_buffer.len(), 0)?;
                buffer.copy_from_slice(eth_buffer);
                Ok(())
            }
            Packet::Ipv4(ref ipv4_buffer) => {
                if raw_type != RawType::Ipv4 {
                    return Err(Error::NoOp);
                }

                buffer.try_resize(ipv4_buffer.len(), 0)?;
                buffer.copy_from_slice(ipv4_buffer);
                Ok(())
            }
        })
    }
}
