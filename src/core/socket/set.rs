use std::iter::Iterator;
use std::slice::IterMut as SliceIterMut;

use {
    Error,
    Result,
};
use core::socket::TaggedSocket;

/// A set of sockets with stable integral handles.
pub struct SocketSet {
    sockets: Vec<Option<TaggedSocket>>,
}

impl SocketSet {
    /// Creates a socket set supporting a maximum number of sockets.
    pub fn new(socket_capacity: usize) -> SocketSet {
        SocketSet {
            sockets: (0 .. socket_capacity).map(|_| None).collect(),
        }
    }

    /// Adds a socket and returns a stable handle.
    pub fn add_socket(&mut self, socket: TaggedSocket) -> Result<usize> {
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
    pub fn socket(&mut self, socket_handle: usize) -> &mut TaggedSocket {
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
    pub fn iter_mut<'a>(&'a mut self) -> SocketIter<'a> {
        SocketIter {
            slice_iter: self.sockets.iter_mut(),
        }
    }
}

pub struct SocketIter<'a> {
    slice_iter: SliceIterMut<'a, Option<TaggedSocket>>,
}

impl<'a> Iterator for SocketIter<'a> {
    type Item = &'a mut TaggedSocket;

    fn next(&mut self) -> Option<&'a mut TaggedSocket> {
        while let Some(socket_option) = self.slice_iter.next() {
            if let Some(ref mut socket) = *socket_option {
                return Some(socket);
            }
        }

        None
    }
}
