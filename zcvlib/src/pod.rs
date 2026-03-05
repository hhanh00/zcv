use bech32::{Bech32m, Hrp};
use bincode::Encode;
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
use serde_with::serde_as;

use crate::{ZCVResult, db::derive_spending_key, error::IntoAnyhow, tiu};

pub const ZCV_MNEMONIC_DOMAIN: &[u8] = b"ZCVote__Personal";

// TODO: Remove question level

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ElectionProps {
    pub secret_seed: Option<String>,
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
    pub caption: String,
    pub questions: Vec<QuestionProp>,
}

#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
pub struct QuestionProp {
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    pub answers: Vec<String>,
}

#[serde_as]
#[derive(Clone, Encode, Serialize, Deserialize, Debug)]
pub struct ElectionPropsPub {
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
    pub caption: String,
    pub questions: Vec<QuestionProp>,
    pub address: String,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub domain: Vec<u8>,
}

pub const ZCV_HRP: &str = "zcv";

impl ElectionProps {
    pub fn build(self, secret_seed: &str) -> ZCVResult<ElectionPropsPub> {
        let ElectionProps {
            start,
            end,
            need_sig,
            name,
            caption,
            questions,
            ..
        } = self;
        let hrp = Hrp::parse(ZCV_HRP).anyhow()?;

        let sk = derive_spending_key(
            &zcash_protocol::consensus::Network::MainNetwork,
            secret_seed,
            0,
        )
        .anyhow()?;

        let vk = FullViewingKey::from(&sk);
        let address = vk.address_at(0u64, Scope::External);
        let address =
            bech32::encode::<Bech32m>(hrp, &address.to_raw_address_bytes()).anyhow()?;

        let eph = ElectionPropsHashable {
            start,
            end,
            need_sig,
            name: name.clone(),
            caption: caption.clone(),
            questions: questions.clone(),
        };
        let domain = eph.calculate_domain()?.to_repr().to_vec();

        let e = ElectionPropsPub {
            start,
            end,
            need_sig,
            name,
            caption,
            questions,
            address,
            domain,
        };
        tracing::info!("{}", serde_json::to_string(&e).unwrap());
        Ok(e)
    }
}

#[derive(Clone, Encode, Debug)]
pub struct ElectionPropsHashable {
    pub start: u32,
    pub end: u32,
    pub need_sig: bool,
    pub name: String,
    pub caption: String,
    pub questions: Vec<QuestionProp>,
}

impl ElectionPropsHashable {
    pub fn calculate_domain(&self) -> ZCVResult<Fp> {
        let m = bincode::encode_to_vec(self, bincode::config::standard()).anyhow()?;
        let d = calculate_domain(&m);
        Ok(d)
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
    use crate::{
        pod::ElectionProps,
        tests::{TEST_ELECTION, TEST_ELECTION_HASH},
    };

    #[test]
    fn test_election_parse() {
        let e = TEST_ELECTION;
        let e: ElectionProps = serde_json::from_value(e.clone()).unwrap();
        let epub = e.build("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about").unwrap();

        let domain = &epub.domain;
        println!("{}", hex::encode(domain));
        assert_eq!(domain, TEST_ELECTION_HASH);
    }
}
