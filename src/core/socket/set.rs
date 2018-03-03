use std::iter::Iterator;
use std::slice::IterMut as SliceIterMut;

use {
    Error,
    Result,
};
use core::socket::TaggedSocket;
use core::storage::Slice;

/// A set of sockets with integral handles...
pub struct SocketSet<'a, 'b: 'a> {
    sockets: Slice<'a, Option<TaggedSocket<'b>>>,
}

impl<'a, 'b> SocketSet<'a, 'b> {
    /// Creates a socket set.
    pub fn new(sockets: Slice<'a, Option<TaggedSocket<'b>>>) -> SocketSet<'a, 'b> {
        SocketSet { sockets: sockets }
    }

    /// Adds a socket and returns a stable handle.
    pub fn add_socket(&mut self, socket: TaggedSocket<'b>) -> Result<usize> {
        let handle = {
            (0 .. self.sockets.len())
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

    /// Returns a reference to a socket with the specified handle. Causes a panic
    /// if the handle is not in use.
    pub fn socket(&mut self, socket_handle: usize) -> &mut TaggedSocket<'b> {
        if socket_handle >= self.sockets.len() {
            panic!("Socket handle is not in use.")
        } else {
            match self.sockets[socket_handle] {
                Some(ref mut socket) => socket,
                _ => panic!("Socket handle is not in use."),
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
    slice_iter: SliceIterMut<'a, Option<TaggedSocket<'b>>>,
}

impl<'a, 'b, 'c> Iterator for SocketIter<'a, 'b> {
    type Item = &'a mut TaggedSocket<'b>;

    fn next(&mut self) -> Option<&'a mut TaggedSocket<'b>> {
        while let Some(socket_option) = self.slice_iter.next() {
            if let Some(ref mut socket) = *socket_option {
                return Some(socket);
            }
        }

        None
    }
}
