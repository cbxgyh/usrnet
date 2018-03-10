#!/bin/bash

echo "Provisioning VM..."

sudo apt-get update

# Setup ZSH.
sudo apt-get install git zsh -y
git clone https://github.com/robbyrussell/oh-my-zsh.git /home/vagrant/.oh-my-zsh
sudo chsh -s /bin/zsh vagrant

# Setup core packages.
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
    arping              \
    traceroute          \
    bridge-utils        \
    build-essential     \
    curl                \
    manpages-dev        \
    tmux                \
    tree                \
    tshark

# Setup Rust.
curl -sSL https://sh.rustup.rs -sSf | sh -s -- -y

echo "Done!"
