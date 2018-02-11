use std::io::Write;

use Result;
use core::layers::EthernetFrame;
use core::socket::Buffer;
use core::storage::Ring;

pub type FrameBuffer<'a> = Ring<'a, Buffer<'a>>;

/// Socket for sending and receiving raw ethernet frames.
pub struct RawSocket<'a> {
    send_buffer: FrameBuffer<'a>,
    recv_buffer: FrameBuffer<'a>,
}

impl<'a> RawSocket<'a> {
    /// Creates a socket with the provided send and receive buffers.
    pub fn new(send_buffer: FrameBuffer<'a>, recv_buffer: FrameBuffer<'a>) -> RawSocket<'a> {
        RawSocket {
            send_buffer,
            recv_buffer,
        }
    }

    /// Attempts to enqueue an ethernet frame for sending via a function f that
    /// writes to a provided frame.
    ///
    /// # Errors
    ///
    /// An error occurs if the send buffer is full.
    pub fn send<F, R>(&'a mut self, buffer_len: usize, f: F) -> Result<R>
    where
        F: FnOnce(EthernetFrame<&mut [u8]>) -> R,
    {
        self.send_buffer.enqueue_maybe(|buffer| {
            match buffer.try_resize(buffer_len) {
                Err(err) => return Err(err),
                _ => {}
            };

            for i in 0..buffer_len {
                buffer[i] = 0;
            }

            let eth_frame = EthernetFrame::try_from(&mut buffer[..buffer_len])?;

            Ok(f(eth_frame))
        })
    }

    /// Attempts to dequeue a received ethernet frame from the socket.
    ///
    /// # Errors
    ///
    /// An error occurs if the receive buffer is empty.
    pub fn recv<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(EthernetFrame<&mut [u8]>) -> R,
    {
        loop {
            match self.recv_buffer
                .dequeue_with(|buffer| EthernetFrame::try_from(&mut buffer[..]))
            {
                Err(err) => return Err(err),
                Ok(Ok(eth_frame)) => return Ok(f(eth_frame)),
                _ => continue,
            };
        }
    }

    pub fn send_forward<F, T>(&mut self, _: F)
    where
        F: FnOnce(&EthernetFrame<T>),
        T: AsRef<[u8]>,
    {
        unimplemented!();
    }

    /// Enqueues an ethernet frame for dequeuing by a future call to recv.
    ///
    /// # Errors
    ///
    /// An error occurs if the receive buffer is full.
    pub fn recv_forward(&mut self, eth_frame: &EthernetFrame<&[u8]>) -> Result<()> {
        self.recv_buffer.enqueue_maybe(|buffer| {
            match buffer.try_resize(eth_frame.len()) {
                Err(err) => return Err(err),
                _ => {}
            };

            (&mut buffer[..]).write(eth_frame)?;

            Ok(())
        })
    }
}
