use bech32::{Bech32m, Hrp};
use bincode::Encode;
use bip39::Mnemonic;
use ff::PrimeField;
use orchard::{
    keys::{FullViewingKey, Scope},
    vote::{calculate_domain, derive_question_sk},
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

#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
pub struct ElectionPropsPub {
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
    pub questions: Vec<QuestionPropPub>,
}

#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
pub struct QuestionPropPub {
    pub domain: String,
    pub title: String,
    pub subtitle: String,
    pub answers: Vec<AnswerPub>,
}

#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
pub struct QuestionPropHashable {
    pub title: String,
    pub subtitle: String,
    pub answers: Vec<AnswerPub>,
}

#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
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
        let questions = questions
            .into_iter()
            .enumerate()
            .map(move |(iq, q)| {
                let answers = q
                    .answers
                    .into_iter()
                    .enumerate()
                    .map(move |(ia, a)| {
                        let sk = derive_question_sk(
                            &s,
                            zcash_protocol::constants::mainnet::COIN_TYPE,
                            iq,
                            ia,
                        )
                        .anyhow();
                        let vk = sk.map(|sk| FullViewingKey::from(&sk));
                        let address = vk.map(|vk| vk.address_at(0u64, Scope::External));
                        let address = address.and_then(|address| {
                            bech32::encode::<Bech32m>(hrp, &address.to_raw_address_bytes()).anyhow()
                        });
                        address.map(|address| AnswerPub { value: a, address })
                    })
                    .collect::<anyhow::Result<Vec<_>>>();
                answers.and_then(|answers| {
                    let q = QuestionPropHashable {
                        title: q.title,
                        subtitle: q.subtitle,
                        answers,
                    };
                    let d = bincode::encode_to_vec(&q, bincode::config::standard());
                    let domain = d
                        .map(|d| calculate_domain(&d))
                        .map(|d| hex::encode(d.to_repr()));
                    let QuestionPropHashable {
                        title,
                        subtitle,
                        answers,
                    } = q;
                    domain
                        .map(|domain| QuestionPropPub {
                            domain,
                            title,
                            subtitle,
                            answers,
                        })
                        .anyhow()
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(ElectionPropsPub {
            start,
            end,
            need_sig,
            name,
            questions,
        })
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
        let epub = e.build().unwrap();
        println!("{epub:?}");
    }
}
