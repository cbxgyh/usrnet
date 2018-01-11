# README

*usrnet* is a user space TCP/IP stack I wrote to learn about networking.

## Building

An Ubuntu dev VM is provided via [Vagrant](https://www.vagrantup.com/)
to build *usrnet* and run the provided examples. Just...

1. `vagrant up && vagrant ssh`
2. `cd /usrnet && cargo build`

... and that's it!

Note, there are actually two VM's configured in the
[Vagrantfile](/vagrant/Vagrantfile)
which are needed for the examples. You may need to change the static IP's
assigned to these VM's if you get conflicts when running `vagrant up`.

## Examples

The [examples](/examples) contain simplified programs of some common
networking utilities. These examples use a
[Linux TAP](http://backreference.org/2010/03/26/tuntap-interface-tutorial/)
interface to transmit raw ethernet frames across a bridge and through the VM's
ethernet interface.

**This means the examples will only compile on a Linux system!**

Note, if you changed the static IP's of the dev VM's you will need to adjust
the IP of the device in each example.

## Resources

- [Stanford's CS 144 MOOC](https://lagunita.stanford.edu/courses/Engineering/Networking-SP/SelfPaced/courseware)
- [Let's Code a TCP/IP Stack](http://www.saminiir.com/lets-code-tcp-ip-stack-1-ethernet-arp/)
- [OpenVPN Tunneling](http://www.saminiir.com/openvpn-puts-packets-inside-your-packets)
- [Linux Virtual Network Devices](http://blog.povilasb.com/posts/linux-virtual-network-devices/)
- [TUN/TAP Interface Tutorial](http://backreference.org/2010/03/26/tuntap-interface-tutorial/)
- [How to Send an Arbitrary Ethernet Frame on Linux with C](http://www.microhowto.info/howto/send_an_arbitrary_ethernet_frame_using_an_af_packet_socket_in_c.html)
- [smoltcp](https://github.com/m-labs/smoltcp)
