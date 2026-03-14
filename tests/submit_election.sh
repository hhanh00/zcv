#!/bin/sh

yq eval -o json "$1.yml" > "$1.json"
ELECTION_SEED="stool rich together paddle together pool raccoon promote attitude peasant latin concert"
./target/release/creator --election-file "$1.json" --seed "$ELECTION_SEED" --output-file "$1-pub.json"
ELECTION=$(cat "$1-pub.json")
ELECTION_REQ=$(jq -n --arg election "$ELECTION" '{"election": $election}')
echo $ELECTION_REQ

grpcurl -d "$ELECTION_REQ" --proto zcvlib/protos/vote.proto --plaintext localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/SetElection
grpcurl -d '{}' --proto zcvlib/protos/vote.proto --plaintext localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/Lock
