//! Communication between endpoints.
//!
//! The `socket` module provides abstractions for buffering, sending, and
//! receiving data between network endpoints.

pub mod bindings;
pub mod env;
pub mod raw;
pub mod set;
pub mod tagged;
pub mod tcp;
pub mod udp;

pub use self::bindings::{
    Bindings,
    SocketAddr,
    SocketAddrLease,
    TaggedSocketAddr,
};
pub use self::env::SocketEnv;
pub use self::raw::{
    RawSocket,
    RawType,
};
pub use self::set::SocketSet;
pub use self::tagged::TaggedSocket;
pub use self::tcp::{
    Tcp,
    TcpClosed,
    TcpContext,
    TcpEstablished,
    TcpListen,
    TcpSocket,
    TcpState,
    TcpSynRecv,
    TcpSynSent,
};
pub use self::udp::UdpSocket;
