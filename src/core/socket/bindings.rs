use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult,
};
use std::ops::Deref;

use {
    Error,
    Result,
};
use core::repr::Ipv4Address;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// An IPv4 + port binding for TCP, UDP, etc. sockets.
pub struct SocketAddr {
    pub addr: Ipv4Address,
    pub port: u16,
}

impl Display for SocketAddr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}:{}", self.addr, self.port)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum TaggedSocketAddr {
    Udp(SocketAddr),
}

/// Represents a borrow of a socket address to ensure sockets are binded to
/// unique addresses.
#[derive(Debug)]
pub struct AddrLease<'a> {
    addr: TaggedSocketAddr,
    owner: &'a Bindings,
}

impl<'a> Deref for AddrLease<'a> {
    type Target = SocketAddr;

    fn deref(&self) -> &SocketAddr {
        match self.addr {
            TaggedSocketAddr::Udp(ref addr) => addr,
        }
    }
}

impl<'a> Drop for AddrLease<'a> {
    fn drop(&mut self) {
        self.owner.bindings.borrow_mut().remove(&self.addr);
    }
}

/// A set of socket bindings.
#[derive(Debug)]
pub struct Bindings {
    bindings: RefCell<HashSet<TaggedSocketAddr>>,
}

impl Bindings {
    /// Creates a set of socket bindings.
    pub fn new() -> Bindings {
        Bindings {
            bindings: RefCell::new(HashSet::new()),
        }
    }

    /// Tries to reserve the specified UDP address returning an Error::InUse
    /// if the binding is already in use.
    pub fn bind_udp(&self, udp_binding: SocketAddr) -> Result<AddrLease> {
        self.bind(TaggedSocketAddr::Udp(udp_binding))
    }

    fn bind(&self, binding: TaggedSocketAddr) -> Result<AddrLease> {
        if self.bindings.borrow_mut().insert(binding.clone()) {
            Ok(AddrLease {
                addr: binding,
                owner: self,
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
        let udp_binding = SocketAddr {
            addr: Ipv4Address::new([0, 1, 2, 3]),
            port: 1024,
        };
        assert_eq!(*bindings.bind_udp(udp_binding).unwrap(), udp_binding);
    }

    #[test]
    fn test_bind_udp_err() {
        let bindings = Bindings::new();
        let udp_binding = SocketAddr {
            addr: Ipv4Address::new([0, 1, 2, 3]),
            port: 1024,
        };
        let _udp_lease = bindings.bind_udp(udp_binding).unwrap();
        assert_matches!(bindings.bind_udp(udp_binding), Err(Error::InUse));
    }
}
