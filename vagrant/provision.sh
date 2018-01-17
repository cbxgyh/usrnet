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
    bridge-utils        \
    build-essential     \
    curl                \
    manpages-dev        \
    tmux                \
    tree                \
    tshark

# Setup Rust.
curl -sSL https://sh.rustup.rs -sSf | sh -s -- -y

# Make tap.sh runs on startup...
sudo cp /usrnet/vagrant/tap.sh /etc/init.d/tap.sh
sudo chmod a+x /etc/init.d/tap.sh
sudo chown root /etc/init.d/tap.sh
sudo update-rc.d tap.sh defaults

echo "Done!"
