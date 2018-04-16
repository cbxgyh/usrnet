use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult,
};
use std::net::SocketAddrV4;
use std::ops::Deref;
use std::rc::Rc;

use {
    Error,
    Result,
};
use core::repr::Ipv4Address;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// An IPv4 + port socket address.
pub struct SocketAddr {
    pub addr: Ipv4Address,
    pub port: u16,
}

impl Display for SocketAddr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}:{}", self.addr, self.port)
    }
}

impl<'a> From<&'a SocketAddrV4> for SocketAddr {
    fn from(socket_addr: &'a SocketAddrV4) -> SocketAddr {
        SocketAddr {
            addr: Ipv4Address::from(socket_addr.ip()),
            port: socket_addr.port(),
        }
    }
}

impl Into<SocketAddrV4> for SocketAddr {
    fn into(self) -> SocketAddrV4 {
        SocketAddrV4::new(self.addr.into(), self.port)
    }
}

/// A socket address corresponding to different socket types.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum TaggedSocketAddr {
    Udp(SocketAddr),
    Tcp(SocketAddr),
}

/// A socket address which has been reserved, and is freed for reallocation by
/// the owning Bindings instance once dropped.
#[derive(Debug, Eq, PartialEq)]
pub struct SocketAddrLease {
    addr: TaggedSocketAddr,
    socket_addrs: Rc<RefCell<HashSet<TaggedSocketAddr>>>,
}

impl Deref for SocketAddrLease {
    type Target = SocketAddr;

    fn deref(&self) -> &SocketAddr {
        match self.addr {
            TaggedSocketAddr::Tcp(ref addr) => addr,
            TaggedSocketAddr::Udp(ref addr) => addr,
        }
    }
}

impl Drop for SocketAddrLease {
    fn drop(&mut self) {
        self.socket_addrs.borrow_mut().remove(&self.addr);
    }
}

/// An allocator for socket address leases.
#[derive(Debug)]
pub struct Bindings {
    socket_addrs: Rc<RefCell<HashSet<TaggedSocketAddr>>>,
}

impl Bindings {
    /// Creates a set of socket bindings.
    pub fn new() -> Bindings {
        Bindings {
            socket_addrs: Rc::new(RefCell::new(HashSet::new())),
        }
    }

    /// Tries to reserve the specified UDP socket address, returning an Error::InUse
    /// if the socket address is already in use.
    pub fn bind_udp(&self, socket_addr: SocketAddr) -> Result<SocketAddrLease> {
        self.bind(TaggedSocketAddr::Udp(socket_addr))
    }

    /// Tries to reserve the specified TCP socket address, returning an Error::InUse
    /// if the socket address is already in use.
    pub fn bind_tcp(&self, socket_addr: SocketAddr) -> Result<SocketAddrLease> {
        self.bind(TaggedSocketAddr::Tcp(socket_addr))
    }

    fn bind(&self, socket_addr: TaggedSocketAddr) -> Result<SocketAddrLease> {
        if self.socket_addrs.borrow_mut().insert(socket_addr.clone()) {
            Ok(SocketAddrLease {
                addr: socket_addr,
                socket_addrs: self.socket_addrs.clone(),
            })
        } else {
            Err(Error::InUse)
        }
    }
}

#[cfg(test)]
mod tests {
    use core::repr::Ipv4Address;

    use super::*;

    #[test]
    fn test_bind_udp_ok() {
        let bindings = Bindings::new();
        let socket_addr = SocketAddr {
            addr: Ipv4Address::new([0, 1, 2, 3]),
            port: 1024,
        };
        assert_eq!(*bindings.bind_udp(socket_addr).unwrap(), socket_addr);
    }

    #[test]
    fn test_bind_udp_err() {
        let bindings = Bindings::new();
        let socket_addr = SocketAddr {
            addr: Ipv4Address::new([0, 1, 2, 3]),
            port: 1024,
        };
        let _addr_lease = bindings.bind_udp(socket_addr).unwrap();
        assert_matches!(bindings.bind_udp(socket_addr), Err(Error::InUse));
    }
}
