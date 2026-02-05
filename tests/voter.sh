#!/bin/bash

# jq -Rs convers the JSON literal into a string
gq http://127.0.0.1:8000/graphql \
-q 'mutation ($electionJson: String!) {storeElection(electionJson: $electionJson)}' \
-v electionJson="$(jq -Rs . <<< '{"start":3155000,"end":3169000,"need_sig":true,"name":"Test Election","questions":[{"title":"Q1. What is your favorite color?","subtitle":"","index":0,"address":"zcv1re3za92mksd4hga0xw6rwxlklkxsqe9nuqqtdws8mu7cynd6gee74863uq4s9aze6q2zywze20y","choices":[{"title":null,"subtitle":null,"answers":["Red","Green","Blue"]}]},{"title":"Q2. Is the earth flat?","subtitle":"","index":1,"address":"zcv1panzgdd6kyygjqtykys6snl9sy59tdnhrpmezdamlt0umxcgs3z4mrndy7eajpkpxerry7tvccv","choices":[{"title":null,"subtitle":null,"answers":["Yes","No"]}]},{"title":"Q3. Do you like pizza?","subtitle":"","index":2,"address":"zcv1yk6u9k8t6087ru4vsjfzepfw9yhhgpnua27r74wmqyqetn35663c62tnfzw46vqqtu2g54jwqt8","choices":[{"title":null,"subtitle":null,"answers":["Yes","No"]}]}]}')"

gq http://127.0.0.1:8000/graphql \
-q 'mutation {setSeed(idAccount: 1 aindex: 0 seed: "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew")}' \

gq http://127.0.0.1:8000/graphql \
-q 'mutation {scanNotes(idAccount: 1 hash: "059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7")}'

BALANCE=$(gq -l --format=json http://127.0.0.1:8000/graphql \
-q 'query ($e: String!) {getBalance(hash: $e, idAccount: 1, idxQuestion: 1)}' \
-v e="059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7" \
| jq -r .data.getBalance)
printf '<%s>\n' "$BALANCE"
[[ $BALANCE == "0.01169078" ]]

gq http://127.0.0.1:8000/graphql \
-q '
mutation {
  vote(idAccount: 1 amount: "0.01" hash: "059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7"
  idxQuestion: 0 voteContent: "000100")
}'
