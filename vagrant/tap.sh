#!/bin/bash

# Sets up a Linux TAP device + Ethernet bridge. Couple of notes on using this...
#
# 1) The TAP wil only receive frames if it is UP. As of Linux kernel 2.6.36 TAP
#    interfaces are UP only if a program has opened the interface. You can use
#    "cargo run --example tap_up -- --tap tap0" to bring tap0 UP.
#
# 2) Do not use the same MAC address for a usrnet device as the TAP. Otherwise
#    the bridge swallows frames (or something like that...)

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

echo "Done!"
