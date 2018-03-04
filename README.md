# README

*usrnet* is a user space TCP/IP stack I wrote to learn about networking.

## Building

An Ubuntu dev VM is provided via [Vagrant](https://www.vagrantup.com/) to build *usrnet* and run the provided examples. Just...

1. `vagrant up && vagrant ssh`
2. `cd /usrnet && cargo build && cargo test`

... and that's it!

## Examples

The [examples directory](/examples) contains simplified versions of some common networking programs. You can run them via `cargo run --example <name> -- <args..>`.

These examples use a [Linux TAP](http://backreference.org/2010/03/26/tuntap-interface-tutorial/) interface to transmit raw ethernet frames. **This means the examples will only run on a Linux system!**

[tap.sh](vagrant/tap.sh) provides a clear explanation of the network topology in use so you can debug any issues you may run into. You can update [env.rs](examples/env.rs) if you wish to change the network topology (e.g. IP address of your device) for running the examples.

## Resources

- [Stanford's CS 144 MOOC](https://lagunita.stanford.edu/courses/Engineering/Networking-SP/SelfPaced/courseware)
- [Let's Code a TCP/IP Stack](http://www.saminiir.com/lets-code-tcp-ip-stack-1-ethernet-arp/)
- [OpenVPN Tunneling](http://www.saminiir.com/openvpn-puts-packets-inside-your-packets)
- [Linux Virtual Network Devices](http://blog.povilasb.com/posts/linux-virtual-network-devices/)
- [TUN/TAP Interface Tutorial](http://backreference.org/2010/03/26/tuntap-interface-tutorial/)
- [How to Send an Arbitrary Ethernet Frame on Linux with C](http://www.microhowto.info/howto/send_an_arbitrary_ethernet_frame_using_an_af_packet_socket_in_c.html)
- [smoltcp](https://github.com/m-labs/smoltcp)
