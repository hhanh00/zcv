use bip39::Mnemonic;
use orchard::keys::SpendingKey;
use pasta_curves::Fp;
use ff::PrimeField;
use sqlx::{SqliteConnection, query, query_as};

use crate::{
    ZCVResult,
    error::IntoAnyhow,
    pod::{ElectionPropsPub, ZCV_MNEMONIC_DOMAIN},
};

impl ElectionPropsPub {
    pub async fn store(&self, conn: &mut SqliteConnection) -> ZCVResult<()> {
        let hash = self.hash()?;
        let json = serde_json::to_string(self).anyhow()?;
        let (election,): (u32,) = query_as(
            "INSERT INTO elections
            (hash, start, end, need_sig, name, data)
            VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT DO UPDATE SET
            start = excluded.start,
            end = excluded.end,
            need_sig = excluded.need_sig,
            name = excluded.name,
            data = excluded.data
            RETURNING id_election",
        )
        .bind(hash.as_slice())
        .bind(self.start)
        .bind(self.end)
        .bind(self.need_sig)
        .bind(&self.name)
        .bind(&json)
        .fetch_one(&mut *conn)
        .await?;
        for (i, q) in self.questions.iter().enumerate() {
            let q_js = serde_json::to_string(q).anyhow()?;
            let domain = q.domain(self)?;
            query(
                "INSERT INTO questions
                (election, idx, domain, title, subtitle, data)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT DO UPDATE SET
                domain = excluded.domain,
                title = excluded.title,
                subtitle = excluded.subtitle,
                data = excluded.data",
            )
            .bind(election)
            .bind(i as u32)
            .bind(domain.to_repr().as_slice())
            .bind(&q.title)
            .bind(&q.subtitle)
            .bind(q_js)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }
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
