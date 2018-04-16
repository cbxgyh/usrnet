use Result;
use core::repr::{
    EthernetFrame,
    Ipv4Address,
    Ipv4Packet,
    UdpPacket,
};
use core::service::Interface;
use core::socket::{
    Bindings,
    RawSocket,
    RawType,
    SocketAddr,
    TcpSocket,
    UdpSocket,
};
use core::storage::{
    Ring,
    Slice,
};
use core::time::Env as TimeEnv;

/// Default number of packets a raw socket can buffer.
pub static RAW_SOCKET_PACKETS: usize = 128;

/// Default number of packets a UDP socket can buffer.
pub static UDP_SOCKET_PACKETS: usize = 128;

/// An environment for creating sockets configured for a particular interface.
pub struct SocketEnv<T: TimeEnv> {
    bindings: Bindings,
    interface_mtu: usize,
    time_env: T,
}

impl<T: TimeEnv> SocketEnv<T> {
    /// Creates a new socket environment.
    pub fn new(interface: &Interface, time_env: T) -> SocketEnv<T> {
        SocketEnv {
            bindings: Bindings::new(),
            interface_mtu: interface.dev.max_transmission_unit(),
            time_env,
        }
    }

    /// Creates a new raw socket.
    pub fn raw_socket(&self, raw_type: RawType) -> RawSocket {
        let header_len = match raw_type {
            RawType::Ethernet => EthernetFrame::<&[u8]>::HEADER_LEN,
            RawType::Ipv4 => {
                EthernetFrame::<&[u8]>::HEADER_LEN + Ipv4Packet::<&[u8]>::MIN_HEADER_LEN
            }
        };

        let payload_len = self.interface_mtu.checked_sub(header_len).unwrap();

        let buffer = || {
            let payload = Slice::from(vec![0; payload_len]);
            Ring::from(vec![payload; RAW_SOCKET_PACKETS])
        };

        RawSocket::new(raw_type, buffer(), buffer())
    }

    /// Creates a new UDP socket.
    pub fn udp_socket(&self, socket_addr: SocketAddr) -> Result<UdpSocket> {
        let binding = self.bindings.bind_udp(socket_addr)?;

        let header_len = EthernetFrame::<&[u8]>::HEADER_LEN + Ipv4Packet::<&[u8]>::MIN_HEADER_LEN
            + UdpPacket::<&[u8]>::HEADER_LEN;

        let payload_len = self.interface_mtu.checked_sub(header_len).unwrap();

        let buffer = || {
            let payload = Slice::from(vec![0; payload_len]);
            let addr = SocketAddr {
                addr: Ipv4Address::new([0, 0, 0, 0]),
                port: 0,
            };
            Ring::from(vec![(payload, addr); UDP_SOCKET_PACKETS])
        };

        Ok(UdpSocket::new(binding, buffer(), buffer()))
    }

    /// Creates a new TCP socket.
    pub fn tcp_socket(&self, socket_addr: SocketAddr) -> Result<TcpSocket<T>> {
        let binding = self.bindings.bind_tcp(socket_addr)?;
        Ok(TcpSocket::new(
            binding,
            self.time_env.clone(),
            self.interface_mtu,
        ))
    }
}
