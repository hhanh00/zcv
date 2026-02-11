import { GraphQLClient, gql } from "graphql-request";
import * as bip39 from "bip39";

const storeElection = gql`
  mutation storeElection($election: String!) {
    storeElection(electionJson: $election)
  }
`;
const election = {
  start: 3155000,
  end: 3169000,
  need_sig: true,
  name: "Test Election",
  questions: [
    {
      title: "Q1. What is your favorite color?",
      subtitle: "",
      index: 0,
      address:
        "zcv1re3za92mksd4hga0xw6rwxlklkxsqe9nuqqtdws8mu7cynd6gee74863uq4s9aze6q2zywze20y",
      choices: [
        { title: null, subtitle: null, answers: ["Red", "Green", "Blue"] },
      ],
    },
    {
      title: "Q2. Is the earth flat?",
      subtitle: "",
      index: 1,
      address:
        "zcv1panzgdd6kyygjqtykys6snl9sy59tdnhrpmezdamlt0umxcgs3z4mrndy7eajpkpxerry7tvccv",
      choices: [{ title: null, subtitle: null, answers: ["Yes", "No"] }],
    },
    {
      title: "Q3. Do you like pizza?",
      subtitle: "",
      index: 2,
      address:
        "zcv1yk6u9k8t6087ru4vsjfzepfw9yhhgpnua27r74wmqyqetn35663c62tnfzw46vqqtu2g54jwqt8",
      choices: [{ title: null, subtitle: null, answers: ["Yes", "No"] }],
    },
  ],
};
const vars = {
  election: JSON.stringify(election),
};
const client = new GraphQLClient("http://localhost:8000/graphql", {});
const rep = await client.request(storeElection, vars);
console.log(rep);
const hash = rep.storeElection;

var accounts = [];
for (var i = 0; i < 50; i++) {
  const seed = bip39.generateMnemonic(256);
  const storeSeed = gql`
    mutation seed($seed: String!, $account: Int!) {
      setSeed(seed: $seed, idAccount: $account, aindex: 0)
    }
  `;
  await client.request(storeSeed, {
    seed,
    account: i + 1,
  });

  const mint = gql`
    mutation mint($hash: String!, $account: Int!) {
      mint(idAccount: $account, amount: 10000.5, idxQuestion: 1, hash: $hash)
    }
  `;
  await client.request(mint, {
    hash,
    account: i + 1,
  });
  accounts.push(i + 1);
}

const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

await sleep(5000);
const scan = gql`
  mutation scan($hash: String!, $accounts: [Int!]!) {
    scanBallots(hash: $hash, idAccounts: $accounts)
  }
`;
await client.request(scan, {
  hash,
  accounts,
});

const vote = gql`
  mutation vote(
    $hash: String!
    $account: Int!
    $amount: BigDecimal!
    $question: Int!
    $answer: String!
  ) {
    vote(
      idAccount: $account
      amount: $amount
      hash: $hash
      idxQuestion: $question
      voteContent: $answer
    )
  }
`;

for (var i = 1; i <= 50; i++) {
  const answerBytes = [i, i + 1, i + 2];

  const answer = answerBytes
    .map((b) => (b & 0xff).toString(16).padStart(2, "0"))
    .join("");

  const vars = {
    account: i,
    amount: i * 10,
    hash,
    question: 1,
    answer,
  };
  await client.request(vote, vars);
}

await sleep(5000);
await client.request(gql`
  mutation ($hash: String!) {
    decodeBallots(
      electionSeed: "stool rich together paddle together pool raccoon promote attitude peasant latin concert"
      hash: $hash
    )
    collectResults
  }
`, { hash });

await sleep(1000);
await client.request(gql`
  mutation  {
    collectResults
  }
`, { hash });
