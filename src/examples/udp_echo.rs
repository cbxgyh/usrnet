use std::time::{
    Duration,
    Instant,
};
use std::vec::Vec;

use core::service::{
    socket,
    Interface,
};
use core::socket::{
    SocketAddr,
    SocketSet,
};

/// Waits for a single UDP packet and echo's it to the sender.
pub fn udp_echo(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    udp_handle: usize,
    timeout: Duration,
) -> Option<()> {
    if let Some((payload, addr)) = recv(interface, socket_set, udp_handle, timeout) {
        // Socket may have a full send buffer!
        while let Err(_) = socket_set
            .socket(udp_handle)
            .as_udp_socket()
            .send(payload.len(), addr)
            .map(|buffer| buffer.copy_from_slice(&payload))
        {
            socket::send(interface, socket_set);
        }

        // Now drain to guarantee the UDP response makes it onto the link.
        while socket_set
            .socket(udp_handle)
            .as_udp_socket()
            .send_enqueued() > 0
        {
            socket::send(interface, socket_set);
        }

        Some(())
    } else {
        None
    }
}

fn recv(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    udp_handle: usize,
    timeout: Duration,
) -> Option<(Vec<u8>, SocketAddr)> {
    let wait_at = Instant::now();
    let mut buf = vec![0; interface.dev.max_transmission_unit()];

    while Instant::now().duration_since(wait_at) < timeout {
        if let Ok((payload, addr)) = socket_set.socket(udp_handle).as_udp_socket().recv() {
            buf.resize(payload.len(), 0);
            buf.copy_from_slice(payload);
            return Some((buf, addr));
        }

        socket::send(interface, socket_set);
        socket::recv(interface, socket_set);
    }

    None
}
