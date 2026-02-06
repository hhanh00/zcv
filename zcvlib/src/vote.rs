use orchard::vote::{Ballot, BallotWitnesses};
use rand_core::OsRng;
use sqlx::SqliteConnection;
use zcash_protocol::consensus::Network;

use crate::{ZCVResult, ballot::{encrypt_ballot_data, encrypt_ballot_data_with_spends}, db::{get_account_address, get_domain}};

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

pub async fn mint(
    network: &Network,
    conn: &mut SqliteConnection,
    hash: &[u8],
    id_account: u32,
    idx_question: u32,
    amount: u64,
) -> ZCVResult<Ballot> {
    let idx_question = idx_question as usize;
    let (domain, _) = get_domain(conn, hash, idx_question).await?;
    let address = get_account_address(network, conn, id_account).await?;

    let data = encrypt_ballot_data_with_spends(
        network, conn, domain, id_account, &address, &[], amount, vec![],
        amount, OsRng,
    )
    .await?;
    Ok(Ballot {
        data,
        witnesses: dummy_witnesses(),
    })
}

pub async fn delegate(
    network: &Network,
    conn: &mut SqliteConnection,
    hash: &[u8],
    id_account: u32,
    idx_question: u32,
    address: &str,
    amount: u64,
) -> ZCVResult<Ballot> {
    let idx_question = idx_question as usize;
    let (domain, _) = get_domain(conn, hash, idx_question).await?;

    let data = encrypt_ballot_data(
        network, conn, domain, id_account, &address, &[], amount,
        OsRng,
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
