extern crate env_logger;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate usrnet;

mod context;

use std::net::{
    Shutdown,
    SocketAddr as StdSocketAddr,
    TcpListener,
    TcpStream,
};
use std::thread;
use std::time::{
    Duration,
    Instant,
};

use usrnet::core::repr::Ipv4Address;
use usrnet::core::socket::{
    SocketAddr,
    TaggedSocket,
};
use usrnet::examples::env;

fn std_socket_addr(socket_addr: StdSocketAddr) -> Option<SocketAddr> {
    match socket_addr {
        StdSocketAddr::V4(socket_addr) => Some(SocketAddr {
            addr: Ipv4Address::new(socket_addr.ip().octets()),
            port: socket_addr.port(),
        }),
        _ => None,
    }
}

fn tcp_active_open(context: &mut context::Context, with_server: bool) {
    let eth0_addr = env::ifr_addr("eth0");

    let client_addr = SocketAddr {
        addr: *env::DEFAULT_IPV4_ADDR,
        port: context::rand_port(),
    };

    let connect_addr = SocketAddr {
        addr: Ipv4Address::new(eth0_addr.octets()),
        port: context::rand_port(),
    };

    let server = if with_server {
        let listener = TcpListener::bind(StdSocketAddr::V4(connect_addr.into())).unwrap();

        // Accept connections until we receive one from our own TcpSocket.
        let server = thread::spawn(move || loop {
            let (stream, socket_addr) = listener.accept().unwrap();
            stream.shutdown(Shutdown::Both).unwrap();
            match std_socket_addr(socket_addr) {
                Some(socket_addr) => if socket_addr == client_addr {
                    break;
                },
                _ => {}
            }
        });

        Some(server)
    } else {
        None
    };

    // Create a TcpSocket.
    let tcp_socket = context.socket_env.tcp_socket(client_addr).unwrap();
    let tcp_handle = context
        .socket_set
        .add_socket(TaggedSocket::Tcp(tcp_socket))
        .unwrap();

    context
        .socket_set
        .socket(tcp_handle)
        .as_tcp_socket()
        .connect(connect_addr);

    while context
        .socket_set
        .socket(tcp_handle)
        .as_tcp_socket()
        .is_establishing()
    {
        env::tick(&mut context.interface, &mut context.socket_set);
    }

    // Check the socket status depending on if we started a server or not.
    let tcp_socket = context.socket_set.socket(tcp_handle).as_tcp_socket();
    match server {
        Some(server) => {
            assert!(tcp_socket.is_connected());
            server.join().unwrap();
        }
        _ => assert!(tcp_socket.is_closed()),
    }
}

#[test]
fn tcp_active_open_ok() {
    context::run(|context| {
        tcp_active_open(context, true);
    });
}

#[test]
fn tcp_active_open_reset() {
    context::run(|context| {
        tcp_active_open(context, false);
    });
}

#[test]
fn tcp_passive_open() {
    context::run(|context| {
        // Create a TcpSocket.
        let server_addr = SocketAddr {
            addr: *env::DEFAULT_IPV4_ADDR,
            port: context::rand_port(),
        };

        let tcp_socket = context.socket_env.tcp_socket(server_addr).unwrap();
        let tcp_handle = context
            .socket_set
            .add_socket(TaggedSocket::Tcp(tcp_socket))
            .unwrap();

        // Start a server with a tiny connection queue.
        context
            .socket_set
            .socket(tcp_handle)
            .as_tcp_socket()
            .listen(2, 2);

        // Create a small herd of clients trying to connect to the server.
        let clients: Vec<_> = (0 .. 4)
            .map(|_| {
                thread::spawn(move || {
                    // Open and immediately close the connection.
                    TcpStream::connect(StdSocketAddr::V4(server_addr.into()))
                        .and_then(|stream| stream.shutdown(Shutdown::Both))
                        .unwrap();
                })
            })
            .collect();

        // Wait for all clients to have been granted a connection.
        let mut connected_clients = 0;
        while connected_clients != 4 {
            if let Some(_) = context
                .socket_set
                .socket(tcp_handle)
                .as_tcp_socket()
                .accept()
            {
                connected_clients += 1;
            }
            env::tick(&mut context.interface, &mut context.socket_set);
        }

        // Let's make sure clients have died.
        for client in clients {
            client.join().unwrap();
        }

        // Let's make sure the server does not receiving any further connections.
        let begin = Instant::now();
        while Instant::now() - begin < Duration::from_secs(1) {
            assert!(
                context
                    .socket_set
                    .socket(tcp_handle)
                    .as_tcp_socket()
                    .accept()
                    .is_none()
            );
            env::tick(&mut context.interface, &mut context.socket_set);
        }
    });
}
