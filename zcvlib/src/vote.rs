use orchard::vote::{Ballot, BallotWitnesses};
use rand_core::OsRng;
use sqlx::SqliteConnection;
use zcash_protocol::consensus::Network;

use crate::{ZCVResult, ballot::encrypt_ballot_data, db::get_domain};

pub async fn vote(
    network: &Network,
    conn: &mut SqliteConnection,
    hash: &[u8],
    id_account: u32,
    idx_question: u32,
    memo: &[u8],
    amount: u64,
) -> ZCVResult<Ballot> {
    let idx_question = idx_question as usize;
    let (domain, address) = get_domain(conn, hash, idx_question).await?;

    let data = encrypt_ballot_data(
        network, conn, domain, id_account, &address, memo, amount, OsRng,
    )
    .await?;
    Ok(Ballot {
        data,
        witnesses: dummy_witnesses(),
    })
}

fn dummy_witnesses() -> BallotWitnesses {
    BallotWitnesses {
        proofs: vec![],
        sp_signatures: None,
        binding_signature: [0u8; 64],
    }
}
