#!/bin/bash

# jq -Rs convers the JSON literal into a string
gq http://127.0.0.1:8000/graphql \
-q 'mutation ($electionJson: String!) {storeElection(electionJson: $electionJson)}' \
-v electionJson="$(jq -Rs . <<< '{"start":3155000,"end":3169000,"need_sig":true,"name":"Test Election","questions":[{"title":"Q1. What is your favorite color?","subtitle":"","index":0,"address":"zcv136783gfhep49wrnj8s4gmmkgc34qguvjcymh9je48xfnm59j05anvhtpqkw6gg5dx5d6jp8k48w","choices":[{"title":null,"subtitle":null,"answers":["Red","Green","Blue"]}]},{"title":"Q2. Is the earth flat?","subtitle":"","index":1,"address":"zcv1qc7x9fl70phqq4cp6q83e8r46yhsmdp25qthxeqywy2fa4asymh2a9v8pv6h2n8djhucwlrc0lz","choices":[{"title":null,"subtitle":null,"answers":["Yes","No"]}]},{"title":"Q3. Do you like pizza?","subtitle":"","index":2,"address":"zcv143m9e3y3fpa2mc2aqgvt48psd8ss3sjkhyse0haa7leepuuupjjfrw0mphyg2ek700xesyydtwh","choices":[{"title":null,"subtitle":null,"answers":["Yes","No"]}]}]}')"

gq http://127.0.0.1:8000/graphql \
-q 'mutation {setSeed(seed: "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew")}' \

gq http://127.0.0.1:8000/graphql \
-q 'mutation {scanNotes(hash: "059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7")}'

BALANCE=$(gq -l --format=json http://127.0.0.1:8000/graphql \
-q 'query ($e: String!) {getBalance(hash: $e, idAccount: 1, idxQuestion: 1)}' \
-v e="059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7" \
| jq -r .data.getBalance)
printf '<%s>\n' "$BALANCE"
[[ $BALANCE == "0.01169078" ]]
