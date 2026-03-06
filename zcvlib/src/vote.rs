use std::{collections::HashMap, sync::LazyLock};

use ff::PrimeField;
use orchard::{
    Address,
    vote::{Ballot, BallotWitnesses, Circuit, ProvingKey, VerifyingKey},
};
use pasta_curves::Fp;
use rand_core::OsRng;
use sqlx::{Row, SqliteConnection, query, sqlite::SqliteRow};
use zcash_protocol::consensus::Network;

use crate::{
    ZCVResult,
    balance::list_unspent_notes,
    ballot::encrypt_ballot_data_with_spends,
    db::{get_account_address, get_account_sk, get_domain, get_election, get_ivks},
    tiu,
};

pub async fn vote(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
    memo: &[u8],
    amount: u64,
) -> ZCVResult<Ballot> {
    let (domain, address) = get_domain(conn).await?;
    send_vote(network, conn, id_account, domain, &address, memo, amount).await
}

pub async fn send_vote(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
    domain: Fp,
    address: &str,
    memo: &[u8],
    amount: u64) -> ZCVResult<Ballot> {
    let (_, recipient) = bech32::decode(address).unwrap();
    let recipient = Address::from_raw_address_bytes(&tiu!(recipient)).unwrap();

    let e = get_election(conn).await?;
    let sk = if e.need_sig {
        Some(get_account_sk(network, conn, id_account).await?)
    } else {
        None
    };
    let (fvk, _, _) = get_ivks(network, conn, id_account).await?;
    let utxos = list_unspent_notes(conn, id_account).await?;
    let notes = utxos
        .into_iter()
        .map(|utxo| {
            let p = utxo.position;
            let n = utxo.to_note(&fvk);
            (n, p)
        })
        .collect::<Vec<_>>();

    // No ORDER BY because we manually sort
    let mut nfs = query("SELECT nf FROM vc_nfs")
        .map(|r: SqliteRow| {
            let nf: Vec<u8> = r.get(0);
            Fp::from_repr(tiu!(nf)).unwrap()
        })
        .fetch_all(&mut *conn)
        .await?;
    nfs.sort();

    // Check that we are not spending a previous nullifier
    let utxos = list_unspent_notes(conn, id_account).await?;
    for note in utxos.iter() {
        let note_nf = Fp::from_repr(tiu!(note.nf.clone())).unwrap();
        tracing::info!("Tx input note candidate: {:?}", note_nf);
        if nfs.contains(&note_nf) {
            panic!("Note should not be already spent");
        }
    }

    let nf_ranges = expand_into_ranges(nfs);

    let cmxs = query("SELECT cmx FROM vc_cmxs ORDER BY id_cmx")
        .map(|r: SqliteRow| {
            let cmx: Vec<u8> = r.get(0);
            Fp::from_repr(tiu!(cmx)).unwrap()
        })
        .fetch_all(&mut *conn)
        .await?;

    let (ballot, _) = orchard::vote::vote(
        domain,
        e.need_sig,
        sk,
        &fvk,
        recipient,
        amount,
        memo,
        &notes,
        &nf_ranges,
        &cmxs,
        OsRng,
        |message, _, _| {
            tracing::info!("{}", message);
        },
        &PK,
        &VK,
    )?;

    Ok(ballot)
}

pub fn expand_into_ranges(nfs: Vec<Fp>) -> Vec<Fp> {
    let mut prev = Fp::zero();
    let mut nf_ranges = vec![];
    for r in nfs {
        // Skip empty ranges when nfs are consecutive
        // (with statistically negligible odds)
        if prev < r {
            // Ranges are inclusive of both ends
            let a = prev;
            let b = r - Fp::one();

            nf_ranges.push(a);
            nf_ranges.push(b);
        }
        prev = r + Fp::one();
    }
    let a = prev;
    let b = Fp::one().neg();

    nf_ranges.push(a);
    nf_ranges.push(b);
    nf_ranges
}

pub async fn mint(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
    amount: u64,
) -> ZCVResult<Ballot> {
    let (domain, _) = get_domain(conn).await?;
    let address = get_account_address(network, conn, id_account).await?;

    let data = encrypt_ballot_data_with_spends(
        network,
        conn,
        domain,
        id_account,
        &address,
        &[],
        amount,
        vec![],
        amount,
        OsRng,
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
    id_account: u32,
    address: &str,
    amount: u64,
) -> ZCVResult<Ballot> {
    let (domain, _) = get_domain(conn).await?;
    send_vote(network, conn, id_account, domain, address, &[], amount).await
}

fn dummy_witnesses() -> BallotWitnesses {
    BallotWitnesses {
        proofs: vec![],
        sp_signatures: None,
        binding_signature: [0u8; 64],
    }
}

pub async fn collect_results(conn: &mut SqliteConnection) -> ZCVResult<Vec<VoteResultItem>> {
    query("DELETE FROM v_final_results")
        .execute(&mut *conn)
        .await?;
    let results = query("SELECT answer, votes FROM v_results")
        .map(|r: SqliteRow| {
            let answer: Vec<u8> = r.get(0);
            let votes: u64 = r.get(1);
            (answer, votes)
        })
        .fetch_all(&mut *conn)
        .await?;
    let mut items: HashMap<VoteResultItem, u64> = HashMap::new();
    for (answer, votes) in results {
        for (i, a) in answer.iter().enumerate() {
            if *a == 0 {
                break;
            }
            let item = VoteResultItem {
                idx_question: i as u32,
                idx_answer: *a,
                votes: 0,
            };
            let e = items.entry(item).or_default();
            *e += votes;
        }
    }
    for (k, v) in items {
        query(
            "INSERT INTO v_final_results
        (idx_question, idx_answer, votes)
        VALUES (?1, ?2, ?3)",
        )
        .bind(k.idx_question)
        .bind(k.idx_answer)
        .bind(v as i64)
        .execute(&mut *conn)
        .await?;
    }
    let counts = query(
        "SELECT idx_question, idx_answer, votes
    FROM v_final_results ORDER BY idx_question, idx_answer",
    )
    .map(|r: SqliteRow| {
        let idx_question: u32 = r.get(0);
        let idx_answer: u8 = r.get(1);
        let votes: u64 = r.get(2);
        VoteResultItem {
            idx_question,
            idx_answer,
            votes,
        }
    })
    .fetch_all(&mut *conn)
    .await?;
    Ok(counts)
}

#[derive(Hash, PartialEq, Eq)]
pub struct VoteResultItem {
    pub idx_question: u32,
    pub idx_answer: u8,
    pub votes: u64,
}

pub static PK: LazyLock<ProvingKey<Circuit>> = LazyLock::new(ProvingKey::build);
pub static VK: LazyLock<VerifyingKey<Circuit>> = LazyLock::new(VerifyingKey::build);

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use crate::{db::get_domain, tests::{get_connection, run_scan, test_setup}};

    #[tokio::test]
    #[serial_test::serial]
    async fn test_vote() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        let (_domain, _address) =
            get_domain(&mut conn).await?;

        // TODO
        Ok(())
    }
}
