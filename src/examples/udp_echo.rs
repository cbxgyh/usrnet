use core::service::Interface;
use core::socket::SocketSet;
use examples::env;

/// Runs a UDP echo server as long as f returns true.
pub fn udp_echo<F: FnMut() -> bool>(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    udp_handle: usize,
    mut f: F,
) {
    let mut buf = vec![];

    while f() {
        env::tick(interface, socket_set);

        let addr = match socket_set.socket(udp_handle).as_udp_socket().recv() {
            Ok((payload, addr)) => {
                buf.resize(payload.len(), 0);
                buf.copy_from_slice(payload);
                addr
            }
            _ => continue,
        };

        // Write response, socket may have a full send buffer!
        while let Err(_) = socket_set
            .socket(udp_handle)
            .as_udp_socket()
            .send(buf.len(), addr)
            .map(|buffer| buffer.copy_from_slice(&buf))
        {
            env::tick(interface, socket_set);
        }
    }

    // Now drain to ensure UDP responses make it onto the link.
    while socket_set
        .socket(udp_handle)
        .as_udp_socket()
        .send_enqueued() > 0
    {
        env::tick(interface, socket_set);
    }
}
