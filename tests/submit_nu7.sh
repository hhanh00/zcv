#!/bin/sh

yq eval -o json ./tests/nu7.yml > ./tests/nu7.json
ELECTION_SEED="stool rich together paddle together pool raccoon promote attitude peasant latin concert"
./target/release/creator --election-file tests/nu7.json --seed "$ELECTION_SEED" --output-file ./tests/nu7-election.json
NU7=$(cat ./tests/nu7-election.json)
ELECTION_REQ=$(jq -n --arg election "$NU7" '{"election": $election}')
echo $ELECTION_REQ

grpcurl -d "$ELECTION_REQ" --proto zcvlib/protos/vote.proto --plaintext localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/SetElection
# 85F0F0E62EA0E7257CCB5E2FE035F0851C8D737A05E14977B1AC06D376520EC4
