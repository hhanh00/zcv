#!/bin/bash

DIR=$(dirname $(readlink -f $0))
pushd $DIR

echo "Store election"
gq http://127.0.0.1:8000/graphql \
-q 'mutation ($electionJson: String!) {storeElection(electionJson: $electionJson)}' \
-v electionJson="$(cat $1.json | jq -Rs .)"

echo "Decode"
gq http://127.0.0.1:8000/graphql \
-q 'mutation ($seed: String!) {decodeBallots(electionSeed: $seed)}' \
-v seed="$2"

echo "Collect"
gq http://127.0.0.1:8000/graphql \
-q '
mutation {
  collectResults {
    idxQuestion
    idxAnswer
    votes
  }
}'

popd
