#!/bin/bash

sudo apt update
sudo apt install -y golang-go
sudo snap install tailscale
sudo tailscale up --auth-key $AUTHKEY --hostname=$(hostname) --reset

go install github.com/cometbft/cometbft/cmd/cometbft@v0.38
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

export PATH=$HOME/go/bin:$PATH

wget https://raw.githubusercontent.com/hhanh00/zcv/refs/heads/main/install.sh
chmod +x install.sh

export DIR=$(hostname)
export EXTERNAL_IP=$(tailscale status | grep $DIR | cut -d' ' -f1)

./install.sh download
./install.sh set-node-config
./install.sh start
