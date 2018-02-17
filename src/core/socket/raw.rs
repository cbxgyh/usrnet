use std::io::Write;

use Result;
use core::layers::EthernetFrame;
use core::storage::{
    Ring,
    Slice,
};

pub type FrameBuffer<'a> = Ring<'a, Slice<'a, u8>>;

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
    pub fn send<F, R>(&mut self, payload_len: usize, f: F) -> Result<R>
    where
        F: FnOnce(EthernetFrame<&mut [u8]>) -> R,
    {
        self.send_buffer.enqueue_maybe(|buffer| {
            let eth_frame_len = EthernetFrame::<&[u8]>::buffer_len(payload_len);

            match buffer.try_resize(eth_frame_len, 0) {
                Err(err) => return Err(err),
                _ => {}
            };

            for i in 0..eth_frame_len {
                buffer[i] = 0;
            }

            let eth_frame = EthernetFrame::try_from(&mut buffer[..eth_frame_len])?;

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
        F: FnOnce(EthernetFrame<&[u8]>) -> R,
    {
        loop {
            match self.recv_buffer
                .dequeue_with(|buffer| EthernetFrame::try_from(&buffer[..]))
            {
                Err(err) => return Err(err),
                Ok(Ok(eth_frame)) => return Ok(f(eth_frame)),
                _ => continue,
            };
        }
    }

    /// Attempts to dequeue an ethernet frame enqueued for sending via a
    /// function f which can process the frame.
    ///
    /// # Errors
    ///
    /// An error occurs if the send buffer is empty or f returns an error, in
    /// which case the frame is not dequeued from the socket.
    pub fn send_forward<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(EthernetFrame<&[u8]>) -> Result<R>,
    {
        self.send_buffer.dequeue_maybe(|buffer| {
            let eth_frame = EthernetFrame::try_from(&buffer[..])?;
            f(eth_frame)
        })
    }

    /// Enqueues an ethernet frame for dequeuing by a future call to recv.
    ///
    /// # Errors
    ///
    /// An error occurs if the receive buffer is full.
    pub fn recv_forward(&mut self, eth_frame: &EthernetFrame<&[u8]>) -> Result<()> {
        self.recv_buffer.enqueue_maybe(|buffer| {
            match buffer.try_resize(eth_frame.len(), 0) {
                Err(err) => return Err(err),
                _ => {}
            };

            (&mut buffer[..]).write(eth_frame)?;

            Ok(())
        })
    }
}
