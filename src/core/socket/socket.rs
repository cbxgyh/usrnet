use Result;

pub enum Packet<'a> {
    Raw(&'a mut [u8]),
    Ipv4(&'a mut [u8]),
}

/// A generic interface for processing socket packets.
pub trait Socket {
    /// Attempts to dequeue a packet enqueued for sending via a function f
    /// which can process the packet. If f returns an error, the packet
    /// should **NOT** be dequeued.
    ///
    /// # Errors
    ///
    /// An Error::Exhausted error occurs if the socket has no pending packets
    /// or another error if f returns an error.
    fn send_forward<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(Packet) -> Result<R>;

    /// Provides a packet which the socket **MAY** enqueue for dequeueing in the
    /// future. The packet may be ignored for reasons including an exhausted buffer
    /// or disintrest in the packet.
    ///
    /// # Errors
    ///
    /// An Error::Exhausted error occurs if the socket buffer is full or Error::NoOp
    /// if the packet was ignored.
    fn recv_forward(&mut self, packet: &Packet) -> Result<()>;
}
