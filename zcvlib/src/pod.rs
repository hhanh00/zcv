use bech32::Hrp;
use bip39::Mnemonic;
use orchard::{
    Address,
    keys::{FullViewingKey, Scope, SpendingKey},
    vote::derive_question_sk,
};
use serde::{Deserialize, Serialize};

use crate::{ZCVResult, error::IntoAnyhow};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ElectionProps {
    pub secret_seed: String,
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
    pub questions: Vec<QuestionProp>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct QuestionProp {
    pub title: String,
    pub subtitle: String,
    pub answers: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ElectionPropsPub {
    pub id: String,
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
    pub questions: Vec<QuestionPropPub>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct QuestionPropPub {
    pub title: String,
    pub subtitle: String,
    pub answers: Vec<AnswerPub>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AnswerPub {
    pub value: String,
    pub address: String,
}

impl ElectionProps {
    pub fn build(self) -> ZCVResult<ElectionPropsPub> {
        let ElectionProps {
            secret_seed,
            start,
            end,
            need_sig,
            name,
            questions,
        } = self;
        let hrp = Hrp::parse("zcv").anyhow()?;

        let m = Mnemonic::parse(&secret_seed).anyhow()?;
        let s = m.to_seed("ZCVote");
        let r = questions.into_iter().enumerate().map(move |(iq, q)| {
            q.answers.into_iter().enumerate().map(move |(ia, a)| {
                let sk =
                    derive_question_sk(&s, zcash_protocol::constants::mainnet::COIN_TYPE, iq, ia)
                        .anyhow();
                let vk = sk.map(|sk| FullViewingKey::from(&sk));
                let address = vk.map(|vk| vk.address_at(0u64, Scope::External));
                let address = address.map(|address|
                    bech32::encode(hrp, &address.to_raw_address_bytes()));
                AnswerPub { value: a, address }
            })
            .collect::<anyhow::Result<Vec<_>>>()
        }).collect::<anyhow::Result<Vec<_>>>()?;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::pod::ElectionProps;

    #[test]
    fn test_election_parse() {
        let e = json!({
            "secret_seed": "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "start": 3_000_000,
            "end": 3_100_000,
            "need_sig": true,
            "name": "Test Election",
            "questions": [
                {
                    "title": "Q1. What is your favorite color?",
                    "subtitle": "",
                    "answers": ["Red", "Green", "Blue"]
                },
                {
                    "title": "Q2. Is the earth flat?",
                    "subtitle": "",
                    "answers": ["Yes", "No"]
                },
                {
                    "title": "Q3. Do you like pizza?",
                    "subtitle": "",
                    "answers": ["Yes", "No"]
                },
            ]
        });
        let e: ElectionProps = serde_json::from_value(e).unwrap();
        println!("{e:?}");
    }
}
