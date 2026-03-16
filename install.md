# Installation Notes

This document describes the installation of a cluster of
(validator) nodes for the purpose of running an election/vote.

## Overview

The Coin Voting Software (CVS) uses a CometBFT blockchain to
validate and store the ballots. Every election has a new blockchain
with potentially a different set of validators.

The process is as follows:
- The first validator (ie the coordinator) creates a new
genesis file and bootstraps the blockchain. Blocks are produced
but they are essentially empty and he is the only validator
- He sets the election parameters
- He uploads the genesis file to the cloud
- He gives the url of the genesis file and his node url to
the other participants
- The other nodes download and install the CVS
- They download the genesis file and join the blockchain. They receive
blocks but do not propose or vote on new blocks yet
- Validator nodes get promoted from standard nodes
- Once all the validators have been set, the blockchain is locked and no further modification is allowed
- Voting begins

## Required Hardware / Software

To run a node, you need a Linux machine with the ability to receive
incoming connection on port 26656.

We suggest using Tailscale because it makes port forwarding easier
but it is not a requirement.

The installation script runs with `bash` and your system
should have: `pkill`, `curl`, `jq`, `grpcurl` and the GoLang.

## CometBFT and GRPCurl
You need cometbft 0.38 (not 1.01). Since it is not available as
a downloadable binary, you've got to build it.

```
go install github.com/cometbft/cometbft/cmd/cometbft@v0.38
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest
```

## Cheat Sheet

### Coordinator
Let's say
- your external IP is `100.78.211.68` (from `tailscale status`),
- your install dir is `seed` (it's the name of the directory
where the script installs its files, you pick it)
- the election json file is `election-pub.json` (it was provided
by the election organizer)

```
./install.sh download --dir seed --external-ip 100.78.211.68
./install.sh coordinate --dir seed --external-ip 100.78.211.68
./install.sh start --dir seed --external-ip 100.78.211.68
<wait about 30s for the node to start>
./install.sh set-election --dir seed --external-ip 100.78.211.68 \
  --election-json election-pub.json
```

After the `coordinate` command, you need to upload `seed/cometbft/config/genesis.json` to dropbox and get the download link.

Mine is `https://www.dropbox.com/scl/fi/tykpmy3xmdzlt5tofbsyz/genesis.json?rlkey=sxpltrexkpgwxcmm5cero8rci&st=oylap0zi&dl=1`

Don't forget to change `dl=0` to `dl=1`

There was also a seed node URL printed out. Mine was
`34606da3a8ea806eda0f19b19c5b8b85e4d3e760@100.78.211.68:26656`

These two URLs should be given to the other node validators.

The `run` command starts the node. It runs in the background. To check
its progress, run `tail -f seed/vote.log`.

### Validators and Followers

First join as a follower:

Let's say
- the external IP is 100.104.174.21
- the installation dir you picked is `follow`

```
./install.sh download --dir follow --external-ip 100.104.174.21
./install.sh set-node-config --dir follow --external-ip 100.104.174.21 \
--seed 34606da3a8ea806eda0f19b19c5b8b85e4d3e760@100.78.211.68:26656 \
--genesis-url "https://www.dropbox.com/scl/fi/tykpmy3xmdzlt5tofbsyz/genesis.json?rlkey=sxpltrexkpgwxcmm5cero8rci&st=oylap0zi&dl=1"
./install.sh run --dir seed --external-ip 100.78.211.68
```

The node will start catching up from the seeder. You can check the progress by `tail -f follow/vote.log`

Once your node is synchronized, you may promote to validator.

- Display the current set of validators:
```
./install.sh show-validators --dir follow --external-ip 100.104.174.21
```
- Promote your node to a validator
```
./install.sh promote --dir follow --external-ip 100.104.174.21
```

If you check the validator set again, it should now have your node.

### Locking against changes

Before the election opens, the coordinator must lock against updates.
After this, new nodes cannot become validators (but they can join as
followers), and the election parameters cannot be changed.

```
./install.sh lock --dir seed --external-ip 100.78.211.68
```

Locking can be done by any validator and cannot be reversed.

## Voting
Any node is a gateway to the election system, ie. the voting
wallet app (Zkool) can connect to any of them on their port
9010.

For better privacy, it is highly recommended to add TLS termination
so that the traffic is encrypted, for example by using NGINX
as a reverse proxy.
