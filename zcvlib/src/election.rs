use bip39::Mnemonic;
use orchard::keys::SpendingKey;
use pasta_curves::Fp;
use ff::PrimeField;

use crate::{
    ZCVResult,
    error::IntoAnyhow,
    pod::{ElectionPropsPub, ZCV_MNEMONIC_DOMAIN},
};

impl ElectionPropsPub {
}

pub fn derive_question_sk(seed: &str, coin_type: u32, domain: Fp) -> ZCVResult<SpendingKey> {
    let mnemonic = Mnemonic::parse(seed).anyhow()?;
    let seed = mnemonic.to_seed("");
    let seed = blake2b_simd::Params::new()
        .hash_length(64)
        .personal(ZCV_MNEMONIC_DOMAIN)
        .key(domain.to_repr().as_slice())
        .hash(&seed);
    let spk = SpendingKey::from_zip32_seed(seed.as_array(), coin_type, zip32::AccountId::ZERO).anyhow()?;
    Ok(spk)
}
