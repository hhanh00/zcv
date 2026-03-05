#!/bin/bash

DIR=$(dirname $(readlink -f $0))
pushd $DIR

echo "Store election"
gq http://127.0.0.1:8000/graphql \
-q 'mutation ($electionJson: String!) {storeElection(electionJson: $electionJson)}' \
-v electionJson="$(cat test_election-pub.json | jq -Rs .)"

echo "Set seed"
gq http://127.0.0.1:8000/graphql \
-q 'mutation {setSeed(idAccount: 1 aindex: 0 seed: "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew")}'

echo "Scan"
gq http://127.0.0.1:8000/graphql \
-q 'mutation {scanNotes(idAccounts: [1])}'

echo "Balance"
BALANCE=$(gq -l --format=json http://127.0.0.1:8000/graphql \
-q 'query {getBalance(idAccount: 1)}' \
| jq -r .data.getBalance)
printf '<%s>\n' "$BALANCE"
[[ $BALANCE == "0.01169078" ]]

echo "Vote"
gq http://127.0.0.1:8000/graphql \
-q '
mutation {
  vote(idAccount: 1 amount: "0.01"
  voteContent: "020101")
}'

sleep 5

echo "Scan"
gq http://127.0.0.1:8000/graphql \
-q 'mutation {scanBallots(idAccounts: [1])}'

echo "Balance"
BALANCE=$(gq -l --format=json http://127.0.0.1:8000/graphql \
-q 'query {getBalance(idAccount: 1)}' \
| jq -r .data.getBalance)
printf '<%s>\n' "$BALANCE"
[[ $BALANCE == "0.00169078" ]]

echo "2nd Vote"
gq http://127.0.0.1:8000/graphql \
-q '
mutation {
  vote(idAccount: 1 amount: "0.00169078"
  voteContent: "010202")
}'

popd
