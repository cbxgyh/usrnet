use core::service::Interface;
use core::socket::SocketSet;
use examples::env;

/// Runs a TCP echo server as long as f returns true.
pub fn tcp_echo<F: FnMut() -> bool>(
    interface: &mut Interface,
    socket_set: &mut SocketSet,
    tcp_handle: usize,
    mut f: F,
) {
    socket_set.socket(tcp_handle).as_tcp_socket().listen(16, 16);

    while f() {
        env::tick(interface, socket_set);

        if let Some(_) = socket_set.socket(tcp_handle).as_tcp_socket().accept() {
            debug!("Got a connection!");
        }
    }
}
