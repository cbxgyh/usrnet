#!/bin/bash

# Sets up a Linux TAP device. Note that pinging 10.0.0.1 will forward to the
# loopback interface! You should ping 10.0.0.{2, 3 ...} instead.

sudo ip tuntap add name tap0 mode tap user $USER
sudo ip link set tap0 up
sudo ip addr add 10.0.0.1/24 dev tap0
