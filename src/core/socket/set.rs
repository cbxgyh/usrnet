use std;

use {
    Error,
    Result,
};
use core::socket::Socket;
use core::storage::Slice;

/// A set of sockets with integral handles...
pub struct SocketSet<'a, 'b: 'a> {
    sockets: Slice<'a, Option<Socket<'b>>>,
}

impl<'a, 'b> SocketSet<'a, 'b> {
    /// Creates a socket set.
    pub fn new(sockets: Slice<'a, Option<Socket<'b>>>) -> SocketSet<'a, 'b> {
        SocketSet { sockets: sockets }
    }

    /// Adds a socket and returns a stable handle.
    pub fn add_socket(&mut self, socket: Socket<'b>) -> Result<usize> {
        let handle = {
            (0..self.sockets.len())
                .filter(|i| self.sockets[*i].is_none())
                .next()
        };

        match handle {
            Some(i) => {
                self.sockets[i] = Some(socket);
                Ok(i)
            }
            _ => Err(Error::Exhausted),
        }
    }

    /// Attempts to return a reference to a socket with the specified handle.
    pub fn socket(&mut self, socket_handle: usize) -> Option<&mut Socket<'b>> {
        if socket_handle >= self.sockets.len() {
            None
        } else {
            match self.sockets[socket_handle] {
                Some(ref mut socket) => Some(socket),
                _ => None,
            }
        }
    }

    /// Returns an iterator over all of the sockets in the set.
    pub fn iter_mut<'c>(&'c mut self) -> SocketIter<'c, 'b> {
        SocketIter {
            slice_iter: self.sockets.iter_mut(),
        }
    }
}

pub struct SocketIter<'a, 'b: 'a> {
    slice_iter: std::slice::IterMut<'a, Option<Socket<'b>>>,
}

impl<'a, 'b, 'c> std::iter::Iterator for SocketIter<'a, 'b> {
    type Item = &'a mut Socket<'b>;

    fn next(&mut self) -> Option<&'a mut Socket<'b>> {
        while let Some(socket_option) = self.slice_iter.next() {
            if let Some(ref mut socket) = *socket_option {
                return Some(socket);
            }
        }

        None
    }
}
