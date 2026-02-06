use std::collections::HashMap;

use orchard::vote::{Ballot, BallotWitnesses};
use rand_core::OsRng;
use sqlx::{Row, SqliteConnection, query, sqlite::SqliteRow};
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
        network, conn, domain, id_account, address, &[], amount,
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

pub async fn collect_results(conn: &mut SqliteConnection) -> ZCVResult<()> {
    let results = query("SELECT question, answer, votes FROM results")
    .map(|r: SqliteRow| {
        let idx_question: u32 = r.get(0);
        let answer: Vec<u8> = r.get(1);
        let votes: u64 = r.get(2);
        (idx_question, answer, votes)
    })
    .fetch_all(&mut *conn)
    .await?;
    let mut items: HashMap<Item, u64> = HashMap::new();
    for (idx_question, answer, votes) in results {
        for (i, a) in answer.iter().enumerate() {
            if *a == 0 { break; }
            let item = Item {
                idx_question,
                idx_sub_question: i as u32,
                idx_answer: *a,
            };
            let e = items.entry(item).or_default();
            *e += votes;
        }
    }
    for (k, v) in items {
        query("INSERT INTO final_results
        (idx_question, idx_sub_question, idx_answer, votes)
        VALUES (?1, ?2, ?3, ?4)")
        .bind(k.idx_question)
        .bind(k.idx_sub_question)
        .bind(k.idx_answer)
        .bind(v as i64)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

#[derive(Hash, PartialEq, Eq)]
struct Item {
    idx_question: u32,
    idx_sub_question: u32,
    idx_answer: u8,
}
