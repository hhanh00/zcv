#!/bin/bash

# Requirements: pkill, curl, go, jq, grpcurl

# --- Usage ---
usage() {
  echo "Usage: $(basename "$0") <command> [options]"
  echo ""
  echo "Commands:"
  echo "  download         Download and install the binaries"
  echo "  set-node-config  Download genesis and configure as a full node"
  echo "  start            Start and join as a standard full node"
  echo "  promote          Promote to validator"
  echo "  show-validators  Show the validator set"
  echo "Coordinator Commands:"
  echo "  coordinate       Configure as coordinator"
  echo "  set-election     Set the Election Definition"
  echo "  lock             Lock the Blockchain against further updates"


  echo ""
  exit 1
}

# --- Require at least one argument ---
if [[ $# -lt 1 ]]; then
  echo "Error: a command is required." >&2
  usage
fi

COMMAND="$1"
shift

# --- Parse named flags ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)
      DIR="$2"; shift 2 ;;
    --election-json)
      ELECTION_JSON="$2"; shift 2 ;;
    --external-ip)
      EXTERNAL_IP="$2"; shift 2 ;;
    --seed)
      SEED="$2"; shift 2 ;;
    --genesis-url)
      GENESIS_URL="$2"; shift 2 ;;
    *)
      echo "Error: unknown option '$1'" >&2
      usage ;;
  esac
done

# --- Validate required flags ---
missing=()
[[ -z "$DIR" ]]          && missing+=("--dir")
[[ -z "$EXTERNAL_IP" ]]  && missing+=("--external-ip")

if [[ ${#missing[@]} -gt 0 ]]; then
  echo "Error: missing required flags: ${missing[*]}" >&2
  usage
fi

#BIN_DIR=$HOME/go/bin
BIN_DIR=./zcv

mkdir -p $DIR/zcv/protos
cd $DIR

case "$COMMAND" in
  download)
    echo "Installing binaries..."
    cp "$(go env GOPATH)/bin/cometbft" zcv/

    curl -L -o zcv/vote-cometbft "https://github.com/hhanh00/zcv/releases/download/zcvlib-v0.5.0/vote-cometbft"
    curl -L -o zcv/protos/vote.proto "https://raw.githubusercontent.com/hhanh00/zcv/refs/tags/zcvlib-v0.5.0/zcvlib/protos/vote.proto"
    chmod +x zcv/vote-cometbft
    $BIN_DIR/cometbft init --home cometbft
    ;;

  set-seed)
    missing=()
    [[ -z "$SEED" ]]         && missing+=("--seed")
    [[ -z "$GENESIS_URL" ]]  && missing+=("--genesis-url")

    if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Error: missing required flags: ${missing[*]}" >&2
    usage
    fi

    curl -L -o cometbft/config/genesis.json "$GENESIS_URL"
    sed -i -e "s#seeds = \"\"#seeds = \"$SEED\"#" cometbft/config/config.toml
    ;;

  coordinate)
    echo "Configure as seeder"
    echo "Upload the $DIR/cometbft/config/genesis.json file to the cloud"
    echo "The seed URL is"
    NODEID=$($BIN_DIR/cometbft show-node-id --home cometbft)
    echo "$NODEID@$EXTERNAL_IP:26656"
    ;;

  set-election)
    echo "Configure the election"
    missing=()
    [[ -z "$ELECTION_JSON" ]] && missing+=("--election-json")

    if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Error: missing required flags: ${missing[*]}" >&2
    usage
    fi

    ELECTION=$(cat "../$ELECTION_JSON")
    ELECTION_REQ=$(jq -n --arg election "$ELECTION" '{"election": $election}')

    grpcurl --plaintext --proto zcv/protos/vote.proto -d "$ELECTION_REQ" localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/SetElection
    ;;

  start)
    echo "Starting node..."
    echo "The node will continue to run"
    echo "Run tail -f $DIR/vote.log to check its status"
    pkill cometbft
    nohup $BIN_DIR/cometbft start --home cometbft </dev/null >/dev/null 2>&1 &
    nohup $BIN_DIR/vote-cometbft </dev/null > vote.log 2>&1 &
    ;;

  promote)
    echo "Promoting to validator..."
    PK=$(cat cometbft/config/priv_validator_key.json | jq .pub_key.value)
    grpcurl --plaintext --proto zcv/protos/vote.proto -d "{\"pub_key\": $PK, \"power\": \"10\"}" localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/AddValidator
    ;;

  lock)
    echo "Locking..."
    grpcurl --plaintext --proto zcv/protos/vote.proto -d "{}" localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/Lock
    ;;

  show-validators)
    curl -s localhost:26657/validators | jq .result
    ;;

  *)
    echo "Error: unknown command '$COMMAND'" >&2
    usage
    ;;
esac
