use bech32::{Bech32m, Hrp};
use bincode::Encode;
use blake2b_simd::Params;
use ff::PrimeField;
use orchard::{
    Note,
    keys::{Diversifier, FullViewingKey, Scope},
    note::{RandomSeed, Rho},
    value::NoteValue,
    vote::calculate_domain,
};
use pasta_curves::Fp;
use serde::{Deserialize, Serialize};

use crate::{ZCVResult, election::derive_question_sk, error::IntoAnyhow, tiu};

pub const ZCV_MNEMONIC_DOMAIN: &[u8] = b"ZCVote__Personal";

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
    pub index: usize,
    pub address: String,
    pub answers: Vec<String>,
}

#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
pub struct QuestionPropHashable {
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
    pub title: String,
    pub subtitle: String,
    pub index: usize,
    pub answers: Vec<String>,
}

pub const ZCV_HRP: &str = "zcv";

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
        let hrp = Hrp::parse(ZCV_HRP).anyhow()?;

        let mut questions_pub = vec![];
        for (iq, q) in questions.into_iter().enumerate() {
            let QuestionProp {
                title,
                subtitle,
                answers,
            } = q;

            let q = QuestionPropHashable {
                start,
                end,
                need_sig,
                name: name.clone(),
                title,
                subtitle,
                index: iq,
                answers: answers.clone(),
            };
            let domain = q.calculate_domain()?;

            let sk = derive_question_sk(
                &secret_seed,
                zcash_protocol::constants::mainnet::COIN_TYPE,
                domain,
            )
            .anyhow()?;

            let vk = FullViewingKey::from(&sk);
            let address = vk.address_at(0u64, Scope::External);
            let address =
                bech32::encode::<Bech32m>(hrp, &address.to_raw_address_bytes()).anyhow()?;

            questions_pub.push(QuestionPropPub {
                title: q.title,
                subtitle: q.subtitle,
                index: iq,
                address,
                answers,
            });
        }
        Ok(ElectionPropsPub {
            start,
            end,
            need_sig,
            name,
            questions: questions_pub,
        })
    }
}

impl QuestionPropHashable {
    pub fn calculate_domain(&self) -> ZCVResult<Fp> {
        let m = bincode::encode_to_vec(self, bincode::config::standard()).anyhow()?;
        let d = calculate_domain(&m);
        Ok(d)
    }
}

impl QuestionPropPub {
    pub fn domain(&self, e: &ElectionPropsPub) -> ZCVResult<Fp> {
        let q = QuestionPropHashable {
            start: e.start,
            end: e.end,
            need_sig: e.need_sig,
            name: e.name.clone(),
            title: self.title.clone(),
            subtitle: self.subtitle.clone(),
            index: self.index,
            answers: self.answers.clone(),
        };
        q.calculate_domain()
    }
}

impl ElectionPropsPub {
    pub fn hash(&self) -> ZCVResult<[u8; 32]> {
        let mut hasher = Params::new()
            .personal(b"ZcashElectionHsh")
            .hash_length(32)
            .to_state();
        for q in self.questions.iter() {
            let domain = q.domain(self)?;
            hasher.update(domain.to_repr().as_slice());
        }
        let h: [u8; 32] = tiu!(hasher.finalize().as_bytes());
        Ok(h)
    }
}

pub struct UTXO {
    pub height: u32,
    pub scope: u32,
    pub position: u32,
    pub nf: Vec<u8>,
    pub dnf: Vec<u8>,
    pub rho: Vec<u8>,
    pub diversifier: Vec<u8>,
    pub rseed: Vec<u8>,
    pub value: u64,
}

impl UTXO {
    pub fn to_note(self, fvk: &FullViewingKey) -> Note {
        let UTXO {
            scope,
            rho,
            diversifier,
            rseed,
            value,
            ..
        } = self;
        let d = Diversifier::from_bytes(tiu!(diversifier));
        let scope = if scope == 0 {
            Scope::External
        } else {
            Scope::Internal
        };
        let recipient = fvk.address(d, scope);
        let value = NoteValue::from_raw(value);
        let rho = Rho::from_bytes(&tiu!(rho)).unwrap();
        let rseed = RandomSeed::from_bytes(tiu!(rseed), &rho).unwrap();

        Note::from_parts(recipient, value, rho, rseed).unwrap()
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
        let epub = e.build().unwrap();

        let hash = epub.hash().unwrap();
        assert_eq!(
            hash,
            *hex::decode("8e3fbdd6b559040f6d6d0da90ca75605c0c6f8acd96570b00b8ff9475244e7e1")
                .unwrap()
        );
    }
}
