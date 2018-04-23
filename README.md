# usrnet

*usrnet* is a user space TCP/IP stack I'm writing to learn about networking. It is heavily inspired by [smoltcp](https://github.com/m-labs/smoltcp), an awesome well documented network stack which I used as a guide when building *usrnet*.

## Building

An Ubuntu dev VM is provided via [Vagrant](https://www.vagrantup.com/) to build *usrnet* and run the provided examples. Just...

1. `vagrant up && vagrant ssh`
2. `cd /usrnet && cargo build && cargo test`

... and that's it!

## Examples

The [examples](/examples) directory contains simplified versions of some common networking programs. You can run them via `cargo run --example <name> -- <args..>`. As a basic sanity check you can run the dev_up example and issue a ping to 10.0.0.102 (default IP for example devices) and see if you get a response.

These examples use a [Linux TAP](http://backreference.org/2010/03/26/tuntap-interface-tutorial/) interface to transmit raw ethernet frames. **This means the examples will only run on a Linux system!**

[tap.sh](vagrant/tap.sh) provides a clear explanation of the network topology in use so you can debug any issues you may run into. You can update [env.rs](src/examples/env.rs) if you wish to change the network topology (e.g. IP address of your device) for running the examples.

Check out the [documentation](https://andreimaximov.github.io/usrnet-docs) for more info.

## Tests

In addition to unit tests, the [tests](/tests) directory contains smoke tests for some sample programs. **These tests will only run successfully on a Linux system** for the same reason as the examples. When developing on a different system, you can use `cargo test --lib` to avoid running these tests.

## Features

I'm writing *usrnet* for learning purposes so it supports **only the most basic features**, many of which are not complete but are listed under [Upcoming](#upcoming). Feel free to open an issue if you find a bug in an existing feature!

### Ethernet

- Uses Ethernet II frames for link layer
- Supports unicast and broadcast Ethernet frames
- Supports using and responding to ARP for IP/Ethernet address mapping
- Supports Raw Ethernet sockets for writing programs like [arping](/src/examples/arping.rs)

### IPv4

- Uses default options for IPv4 headers found [here](/src/core/repr/ipv4.rs)
- Supports a default gateway for routing to the internet
- Supports ping with ICMP echo request/reply messages
- Supports Raw IPv4 sockets for writing programs like [ping](/src/examples/ping.rs)

### UDP

- Supports UDP sockets for writing programs like [UDP echo servers](/src/examples/udp_echo.rs)
- Supports traceroute with ICMP destination unreachable responses to UDP packets with an unbound port

### Upcoming

- DNS lookup
- DHCP address assignment
- TCP sockets

## Resources

- [Stanford's CS 144 MOOC](https://lagunita.stanford.edu/courses/Engineering/Networking-SP/SelfPaced/courseware)
- [Let's Code a TCP/IP Stack](http://www.saminiir.com/lets-code-tcp-ip-stack-1-ethernet-arp/)
- [OpenVPN Tunneling](http://www.saminiir.com/openvpn-puts-packets-inside-your-packets)
- [Linux Virtual Network Devices](http://blog.povilasb.com/posts/linux-virtual-network-devices/)
- [TUN/TAP Interface Tutorial](http://backreference.org/2010/03/26/tuntap-interface-tutorial/)
- [How to Send an Arbitrary Ethernet Frame on Linux with C](http://www.microhowto.info/howto/send_an_arbitrary_ethernet_frame_using_an_af_packet_socket_in_c.html)
- [smoltcp](https://github.com/m-labs/smoltcp)
