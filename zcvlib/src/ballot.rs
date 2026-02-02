use std::collections::HashMap;

use bech32::{Bech32m, Hrp};
use ff::PrimeField;
use orchard::{
    Address,
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
    vote::{BallotAnchors, BallotData, encrypt_ballot_action, try_decrypt_ballot},
};
use pasta_curves::Fp;
use rand_core::{CryptoRng, RngCore};
use sqlx::{SqliteConnection, query, query_as};
use zcash_protocol::consensus::{Network, NetworkConstants};

use crate::{
    ZCVError, ZCVResult,
    balance::list_unspent_notes,
    db::{fetch_ballots, get_ivks, store_received_note},
    election::derive_question_sk,
    error::IntoAnyhow,
    pod::ZCV_HRP,
    tiu,
};

pub async fn encrypt_ballot_data<R: CryptoRng + RngCore>(
    network: &Network,
    conn: &mut SqliteConnection,
    domain: Fp,
    address: &str,
    memo: &[u8],
    amount: u64,
    mut rng: R,
) -> ZCVResult<BallotData> {
    let (fvk, _, _) = get_ivks(network, conn, 0).await?;
    let (_, recipient) = bech32::decode(address).anyhow()?;
    let recipient = Address::from_raw_address_bytes(&tiu!(recipient)).unwrap();
    let mut a = amount;
    let utxos = list_unspent_notes(conn, domain).await?;
    let mut spends = vec![];
    for utxo in utxos {
        let u = a.min(utxo.value);
        a -= u;
        spends.push(utxo);
        if a == 0 {
            break;
        }
    }
    if a > 0 {
        return Err(ZCVError::NotEnoughVotes);
    }

    let mut actions = vec![];
    let mut a = amount;
    for spend in spends {
        let spend = spend.to_note(&fvk);
        let spend_amount = a.min(spend.value().inner());
        let (action, _, _) =
            encrypt_ballot_action(domain, fvk.clone(), &spend, recipient, a, memo, &mut rng)?;
        a -= spend_amount;
        actions.push(action);
    }
    assert_eq!(a, 0);
    let ballot = BallotData {
        version: 1,
        domain: domain.to_repr(),
        actions,
        anchors: BallotAnchors {
            nf: [0u8; 32],
            cmx: [0u8; 32],
        },
    };
    Ok(ballot)
}

#[allow(clippy::too_many_arguments)]
pub async fn decrypt_ballot_data(
    conn: &mut SqliteConnection,
    fvk: FullViewingKey,
    domain: Fp,
    question: u32,
    height: u32,
    position: u32,
    ballot: BallotData,
) -> ZCVResult<()> {
    let ivk = fvk.to_ivk(Scope::External);
    let ivk = PreparedIncomingViewingKey::new(&ivk);
    for (i, action) in ballot.actions.into_iter().enumerate() {
        if let Some((note, memo)) = try_decrypt_ballot(&ivk, action)? {
            store_received_note(
                conn,
                domain,
                &fvk,
                &note,
                &memo,
                height,
                position + i as u32,
                question,
                0, // ballots are sent to the external address
            )
            .await?;
        }
    }
    Ok(())
}

pub const MAX_CHOICES: usize = 32;
pub const MAX_QUESTIONS: usize = 64;

fn plurality(
    question: u32,
    amount: u64,
    memo: [u8; 512],
    counts: &mut [u64; MAX_CHOICES * MAX_QUESTIONS],
) -> ZCVResult<()> {
    // each byte in the memo is a "vote" for the choice at
    // that offset
    // we do not use the complete memo in this type of voting
    #[allow(clippy::needless_range_loop)]
    for i in 0..MAX_CHOICES {
        let idx = question as usize * MAX_CHOICES + i;
        if memo[i] != 0 {
            counts[idx] += amount;
        }
    }
    Ok(())
}

pub async fn tally_plurality_election(
    network: &Network,
    conn: &mut SqliteConnection,
    election_seed: &str,
    hash: &[u8],
) -> ZCVResult<()> {
    let counts = tally_ballots(
        network,
        conn,
        election_seed,
        hash,
        [0u64; MAX_CHOICES * MAX_QUESTIONS],
        plurality,
    )
    .await?;
    for (i, c) in counts.iter().enumerate() {
        if *c == 0 {
            continue;
        }
        let idx_question = i / MAX_CHOICES;
        let idx_votes = i % MAX_CHOICES;
        query(
            "INSERT INTO results(question, answer, votes)
        VALUES (?1, ?2, ?3)
        ON CONFLICT DO UPDATE SET
        votes = votes + excluded.votes",
        )
        .bind(idx_question as u32)
        .bind(idx_votes as u32)
        .bind(*c as i64)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn tally_ballots<R>(
    network: &Network,
    conn: &mut SqliteConnection,
    election_seed: &str,
    hash: &[u8],
    mut result: R,
    memo_handler: impl Fn(u32, u64, [u8; 512], &mut R) -> ZCVResult<()>,
) -> ZCVResult<R> {
    let domains: Vec<(u32, Vec<u8>, String)> = query_as(
        "SELECT idx, domain, address FROM questions q
    JOIN elections e ON q.election = e.id_election
    WHERE e.hash = ?1 ORDER BY idx",
    )
    .bind(hash)
    .fetch_all(&mut *conn)
    .await?;
    let mut ivks: HashMap<u32, PreparedIncomingViewingKey> = HashMap::new();
    for (idx, domain, address) in domains {
        let domain = Fp::from_repr(tiu!(domain)).unwrap();
        let spk = derive_question_sk(election_seed, network.coin_type(), domain)?;
        let fvk = FullViewingKey::from(&spk);
        let ivk = fvk.to_ivk(Scope::External);
        let pivk = PreparedIncomingViewingKey::new(&ivk);
        let address2 = fvk.address_at(0u64, Scope::External);
        let address2 = bech32::encode::<Bech32m>(
            Hrp::parse(ZCV_HRP).unwrap(),
            &address2.to_raw_address_bytes(),
        )
        .anyhow()?;
        assert_eq!(address, address2);
        ivks.insert(idx, pivk);
    }

    fetch_ballots(conn, async |question, ballot_data| {
        let pivk = &ivks[&question];
        for action in ballot_data.actions.into_iter() {
            if let Some((note, memo)) = try_decrypt_ballot(pivk, action)? {
                let votes = note.value().inner();
                memo_handler(question, votes, memo, &mut result)?;
            }
        }
        Ok(())
    })
    .await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use orchard::{
        keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
        vote::{Ballot, BallotWitnesses, try_decrypt_ballot},
    };
    use rand_core::OsRng;
    use sqlx::{Connection, query, query_as};
    use zcash_protocol::consensus::{MainNetwork, Network, NetworkConstants};

    use crate::{
        ZCVResult,
        ballot::{encrypt_ballot_data, tally_plurality_election},
        db::{get_domain, get_question},
        election::derive_question_sk,
        error::IntoAnyhow,
        tests::{
            TEST_ELECTION_HASH, TEST_ELECTION_SEED, get_connection, run_scan, test_ballot,
            test_setup,
        },
    };

    #[tokio::test]
    #[serial_test::serial]
    async fn test_ballot_encryption() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        let (domain, address) =
            get_domain(&mut conn, TEST_ELECTION_HASH, 2 /* question index */).await?;
        let ballot = encrypt_ballot_data(
            &Network::MainNetwork,
            &mut conn,
            domain,
            &address,
            &[], /* answer */
            100000,
            OsRng,
        )
        .await?;
        let spk =
            derive_question_sk(TEST_ELECTION_SEED, MainNetwork.coin_type(), domain).anyhow()?;
        let fvk = FullViewingKey::from(&spk);
        let ivk = PreparedIncomingViewingKey::new(&fvk.to_ivk(Scope::External));
        let n = try_decrypt_ballot(&ivk, ballot.actions[0].clone())?;
        assert!(n.is_some());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_tally_ballots() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        query("DELETE FROM ballots").execute(&mut *conn).await?;
        query("DELETE FROM results").execute(&mut *conn).await?;
        let mut tx = conn.begin().await?;
        let (domain, address) = get_domain(&mut tx, TEST_ELECTION_HASH, 1).await?;
        let question = get_question(&mut tx, domain).await?;
        query("UPDATE notes SET value = value * 100000000")
            .execute(&mut *tx)
            .await?;
        query("UPDATE spends SET value = value * 100000000")
            .execute(&mut *tx)
            .await?;
        let ballot = test_ballot(&mut tx, domain, &address, &[0, 0, 1, 0]).await?;
        let mut ballot_bytes = vec![];
        ballot.write(&mut ballot_bytes)?;
        for itx in 0..2 {
            query(
                "INSERT INTO ballots(height, itx, question, data, witness)
        VALUES (1, ?1, ?2, ?3, '')",
            )
            .bind(itx)
            .bind(question.index as u32)
            .bind(&ballot_bytes)
            .execute(&mut *tx)
            .await?;
        }
        tally_plurality_election(
            &Network::MainNetwork,
            &mut tx,
            TEST_ELECTION_SEED,
            TEST_ELECTION_HASH,
        )
        .await?;
        let (votes,): (i64,) =
            query_as("SELECT votes FROM results WHERE question = 1 and answer = 2")
                .fetch_one(&mut *tx)
                .await?;
        assert_eq!(votes, 27_000_000_000_000); // 2 ballots of 135_000
        query("UPDATE notes SET value = value / 100000000")
            .execute(&mut *tx)
            .await?;
        query("UPDATE spends SET value = value / 100000000")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    #[tokio::test]
    async fn make_ballot_bin() -> ZCVResult<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        let (domain, address) = get_domain(&mut conn, TEST_ELECTION_HASH, 1).await?;
        let ballot_data = encrypt_ballot_data(
            &Network::MainNetwork,
            &mut conn,
            domain,
            &address,
            &[1, 1, 1, 1],
            0,
            OsRng,
        )
        .await?;
        let ballot = Ballot {
            data: ballot_data,
            witnesses: BallotWitnesses {
                proofs: vec![],
                sp_signatures: None,
                binding_signature: [0u8; 64],
            },
        };
        let mut ballot_bytes = vec![];
        ballot.write(&mut ballot_bytes).anyhow()?;
        assert_eq!(ballot_bytes.len(), 907);
        Ok(())
    }
}
