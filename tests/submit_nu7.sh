#!/bin/sh

yq eval -o json ./tests/nu7.yml > ./tests/nu7.json
ELECTION_SEED="stool rich together paddle together pool raccoon promote attitude peasant latin concert"
./target/release/creator --election-file tests/nu7.json --seed "$ELECTION_SEED" --output-file ./tests/nu7-election.json
NU7=$(cat ./tests/nu7-election.json)
ELECTION_REQ=$(jq -n --arg election "$NU7" '{"election": $election}')
echo $ELECTION_REQ

grpcurl -d "$ELECTION_REQ" --proto zcvlib/protos/vote.proto --plaintext localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/SetElection
# 78F74E9A1C6D0FE82F91104548D503D1742E262B6B1346F13945420342F29A21
