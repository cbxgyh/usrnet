#!/bin/bash

# Sets up a Linux TAP device + Ethernet bridge. Couples of notes on this...
#
# 1) Our topology looks roughly like so...
#
#    [     Dev @ 10.0.0.102/24     ]  <->  [        VM tap0           ]
#                                                      |
#                                          [ VM br0 @ 10.0.0.101/24  ]  <->  [ VM IP Forwarding ]
#                                                      |                              |
#    [ Host vboxnet0 @ 10.0.0.1/24 ]  <->  [        VM eth1          ]                |
#                                                                                     |
#    [           Host @ ?          ]  <----------------------------------->  [    VM eth0 @ ?   ]
#
#    Here "Dev" is our usrnet device which runs in a user process on the VM. The
#    Vagrantfile configures a private 10.0.0.0/24 subnet for communication between
#    the host, VM, and this device.
#
#    In this script we create br0, a bridge between tap0 and eth1. This enables
#    communication on the 10.0.0.0/24 subnet but not outside it to the general
#    internet. Thus we enable routing via the "nat" table for the 10.0.0.0/24 subnet
#    and use br0 (10.0.0.101/24) as a default gateway for Dev! In effect, packets
#    arriving at br0 (out default gateway) are forwarded to the internet via eth0.
#
# 2) The tap0 will only receive frames if it is UP. As of Linux kernel 2.6.36 TAP
#    interfaces are UP only if a program has opened the interface. You can use
#    "cargo run --example dev_up" to bring tap0 UP.
#
# 3) Do not use the same MAC address for Dev as tap0. Otherwise br0 swallows frames
#    (or something like that...).

if [ -d /sys/class/net/tap0 ]; then
    exit 0
fi

ETH_IP=$(ip -4 addr show eth1 | grep inet | awk '{ print $2; }')

echo "Creating bridge @ $ETH_IP..."

# Create bridge...
sudo ip link add br0 type bridge

# Setup tap0...
sudo ip tuntap add name tap0 mode tap user $USER
sudo ip link set tap0 up
sudo ip link set tap0 master br0

# Setup eth1...
sudo ip link set dev eth1 down
sudo ip addr flush dev eth1
sudo ip link set dev eth1 up
sudo ip link set eth1 master br0

# Finish setting up bridge...
sudo ip link set dev br0 up
sudo ip addr add $ETH_IP dev br0

# Enabled routing for packets arriving to the bridge which acts as our IP gateway.
sudo iptables -t nat -A POSTROUTING -s 10.0.0.0/24 -j MASQUERADE
sudo sysctl net.ipv4.ip_forward=1

echo "Done!"
