import { GraphQLClient, gql } from "graphql-request";
import * as bip39 from "bip39";
import assert from "node:assert/strict";

const client = new GraphQLClient("http://localhost:8000/graphql", {});
const storeElection = gql`
  mutation storeElection($election: String!) {
    storeElection(electionJson: $election)
  }
`;
const election = {"start":3168000,"end":3169000,"need_sig":true,"name":"Test Election","caption": "Test test test","questions":[{"title":"Q1. What is your favorite color?","subtitle":"","answers":["Red","Green","Blue"]},{"title":"Q2. Is the earth flat?","subtitle":"","answers":["Yes","No"]},{"title":"Q3. Do you like pizza?","subtitle":"","answers":["Yes","No"]}],"address":"zcv1re3za92mksd4hga0xw6rwxlklkxsqe9nuqqtdws8mu7cynd6gee74863uq4s9aze6q2zywze20y","domain":"31e3ae6eca52d324ca3198f8ba2b39dae84a9746941ac507641560185f286437"};
const rep = await client.request(storeElection, {
  election: JSON.stringify(election),
});
const hash = rep.storeElection;
console.log(hash)

const nVoters = 5;
for (var i = 1; i <= nVoters; i++) {
  const seed = bip39.generateMnemonic(256);
  const storeSeed = gql`
    mutation seed($seed: String!, $account: Int!) {
      setSeed(seed: $seed, idAccount: $account, aindex: 0)
    }
  `;
  await client.request(storeSeed, {
    seed,
    account: i,
  });

  const mint = gql`
    mutation mint($account: Int!) {
      mint(idAccount: $account, amount: 10000.5)
    }
  `;
  await client.request(mint, {
    account: i,
  });
}

const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

await sleep(5000);
const scan1 = gql`
  mutation scan($accounts: [Int!]!) {
    scanNotes(idAccounts: $accounts)
  }
`;
await client.request(scan1, {
  accounts: Array.from({ length: nVoters }, (_, i) => i + 1),
});

await sleep(1000);
const scan2 = gql`
  mutation scan($accounts: [Int!]!) {
    scanBallots(idAccounts: $accounts)
  }
`;
await client.request(scan2, {
  accounts: Array.from({ length: nVoters }, (_, i) => i + 1),
});

const vote = gql`
  mutation vote(
    $account: Int!
    $amount: BigDecimal!
    $answer: String!
  ) {
    vote(
      idAccount: $account
      amount: $amount
      voteContent: $answer
    )
  }
`;

const scores: Record<number, Record<number, number>> = {};
for (var i = 1; i <= nVoters; i++) {
  const value = Math.trunc(Math.random() * 100);
  const answerBytes = [];
  for (var j = 0; j < 3; j++) {
    const choice = Math.trunc(Math.random() * 2);
    const v = scores[j] || {};
    v[choice] = value + (v[choice] || 0);
    scores[j] = v;
    answerBytes.push(choice + 1);
  }

  const answer = answerBytes
    .map((b) => (b & 0xff).toString(16).padStart(2, "0"))
    .join("");

  console.log(answer);
  const vars = {
    account: i,
    amount: value,
    answer,
  };
  await client.request(vote, vars);
}

await sleep(5000);
await client.request(
  gql`
    mutation {
      decodeBallots(
        electionSeed: "stool rich together paddle together pool raccoon promote attitude peasant latin concert"
      )
    }
  `,
);

await sleep(1000);
const res = await client.request(
  gql`
    mutation {
      collectResults {
        idxQuestion
        idxAnswer
        votes
      }
    }
  `,
);
console.log(res.collectResults);
console.log(scores);

for (var r of res.collectResults) {
  const { idxQuestion, idxAnswer, votes } = r;
  const s = scores[idxQuestion][idxAnswer - 1];
  assert.equal(s, parseInt(votes));
}
