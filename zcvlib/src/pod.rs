use bech32::{Bech32m, Hrp};
use bincode::Encode;
use bip39::Mnemonic;
use blake2b_simd::Params;
use ff::PrimeField;
use orchard::{
    keys::{FullViewingKey, Scope},
    vote::{calculate_domain, derive_question_sk},
};
use serde::{Deserialize, Serialize};

use crate::{ZCVResult, error::IntoAnyhow, tiu};

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
    pub title: String,
    pub subtitle: String,
    pub answers: Vec<AnswerPub>,
}

#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
pub struct QuestionPropHashable {
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
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
                let QuestionProp {
                    title,
                    subtitle,
                    answers,
                } = q;

                let answers = answers
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
                answers
                    .map(|answers| QuestionPropPub {
                        title,
                        subtitle,
                        answers,
                    })
                    .anyhow()
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

impl QuestionPropHashable {
    pub fn for_question(election: &ElectionPropsPub, index: usize) -> Self {
        let q = &election.questions[index];
        QuestionPropHashable {
            start: election.start,
            end: election.end,
            need_sig: election.need_sig,
            name: election.name.clone(),
            title: q.title.clone(),
            subtitle: q.subtitle.clone(),
            answers: q.answers.clone(),
        }
    }

    pub fn calculate_domain(&self) -> ZCVResult<[u8; 32]> {
        let m = bincode::encode_to_vec(self, bincode::config::standard()).anyhow()?;
        let d = calculate_domain(&m).to_repr();
        Ok(d)
    }
}

impl ElectionPropsPub {
    pub fn hash(&self) -> ZCVResult<[u8; 32]> {
        let mut hasher = Params::new().personal(b"ZcashElectionHsh")
        .hash_length(32)
        .to_state();
        for (i, _) in self.questions.iter().enumerate() {
            let domain = QuestionPropHashable::for_question(self, i).calculate_domain()?;
            hasher.update(domain.as_slice());
        }
        let h: [u8; 32] = tiu!(hasher.finalize().as_bytes());
        Ok(h)
    }
}

pub struct UTXO {
    pub scope: u32,
    pub position: u32,
    pub nf: Vec<u8>,
    pub dnf: Vec<u8>,
    pub rho: Vec<u8>,
    pub diversifier: Vec<u8>,
    pub rseed: Vec<u8>,
    pub value: u64,
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

        let hash = super::QuestionPropHashable::for_question(&epub, 1)
            .calculate_domain()
            .unwrap();
        println!("{}", hex::encode(hash));
        assert_eq!(
            hash,
            *hex::decode("5e7c105f1dc89bc582f53cb1b1ab8e46170562af4061a68a67cf9f474a5c623b")
                .unwrap()
        );
    }
}
