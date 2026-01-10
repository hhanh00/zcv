use std::collections::HashMap;

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
    db::{fetch_ballots, get_election, get_ivks, get_question, store_received_note},
    election::derive_question_sk,
    error::IntoAnyhow,
    tiu,
};

pub async fn encrypt_ballot_data<R: CryptoRng + RngCore>(
    network: &Network,
    conn: &mut SqliteConnection,
    domain: Fp,
    answer: usize,
    amount: u64,
    mut rng: R,
) -> ZCVResult<BallotData> {
    let (fvk, _, _) = get_ivks(network, conn).await?;
    let question = get_question(conn, domain).await?;
    let answer = &question.answers[answer];
    let (_, recipient) = bech32::decode(&answer.address).anyhow()?;
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
            encrypt_ballot_action(domain, fvk.clone(), &spend, recipient, a, &mut rng)?;
        a -= spend_amount;
        actions.push(action);
    }
    assert_eq!(a, 0);
    let ballot = BallotData {
        version: 1,
        domain: domain.to_repr().to_vec(),
        actions,
        anchors: BallotAnchors {
            nf: vec![],
            cmx: vec![],
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
    for (i, action) in ballot.actions.iter().enumerate() {
        if let Some(note) = try_decrypt_ballot(&ivk, action)? {
            store_received_note(
                conn,
                domain,
                &fvk,
                &note,
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

pub async fn tally_ballots(
    network: &Network,
    conn: &mut SqliteConnection,
    election_seed: &str,
    hash: &[u8],
) -> ZCVResult<()> {
    let domains: Vec<(u32, Vec<u8>)> = query_as(
        "SELECT id_question, domain FROM questions q
    JOIN elections e ON q.election = e.id_election
    WHERE e.hash = ?1 ORDER BY idx",
    )
    .bind(hash)
    .fetch_all(&mut *conn)
    .await?;
    let mut ivks: HashMap<u32, PreparedIncomingViewingKey> = HashMap::new();
    for (id_question, domain) in domains {
        let domain = Fp::from_repr(tiu!(domain)).unwrap();
        let spk = derive_question_sk(election_seed, network.coin_type(), domain)?;
        let fvk = FullViewingKey::from(&spk);
        let ivk = fvk.to_ivk(Scope::External);
        let pivk = PreparedIncomingViewingKey::new(&ivk);
        ivks.insert(id_question, pivk);
    }
    let election = get_election(conn, hash).await?;
    let mut addresses: HashMap<Vec<u8>, String> = HashMap::new();
    for (iq, q) in election.questions.iter().enumerate() {
        for (ia, a) in q.answers.iter().enumerate() {
            let (_, address) = bech32::decode(&a.address).anyhow()?;
            let question_ref = format!("{}.{}", iq + 1, ia + 1);
            addresses.insert(address, question_ref);
        }
    }

    let mut counts: HashMap<String, u64> = HashMap::new();
    fetch_ballots(conn, async |question, ballot_data| {
        let pivk = &ivks[&question];
        for action in ballot_data.actions.iter() {
            if let Some(note) = try_decrypt_ballot(pivk, action)? {
                let recipient = note.recipient().to_raw_address_bytes();
                if let Some(question_ref) = addresses.get(recipient.as_slice()) {
                    let c = counts.entry(question_ref.clone()).or_insert(0);
                    *c += note.value().inner();
                }
            }
        }
        Ok(())
    })
    .await?;

    for (question_ref, count) in counts {
        tracing::info!("{question_ref} {count}");
        query(
            "INSERT INTO results(question_ref, votes)
        VALUES (?1, ?2)",
        )
        .bind(&question_ref)
        .bind(count as i64)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use ff::PrimeField;
    use orchard::{
        keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
        vote::try_decrypt_ballot,
    };
    use rand_core::OsRng;
    use sqlx::{query, query_as};
    use zcash_protocol::consensus::{MainNetwork, Network, NetworkConstants};

    use crate::{
        ballot::{encrypt_ballot_data, tally_ballots},
        db::get_domain,
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
        let domain = get_domain(&mut conn, 1, 2 /* question index */).await?;
        let ballot = encrypt_ballot_data(
            &Network::MainNetwork,
            &mut conn,
            domain,
            1, /* answer index */
            100000,
            OsRng,
        )
        .await?;
        let spk =
            derive_question_sk(TEST_ELECTION_SEED, MainNetwork.coin_type(), domain).anyhow()?;
        let fvk = FullViewingKey::from(&spk);
        let ivk = PreparedIncomingViewingKey::new(&fvk.to_ivk(Scope::External));
        let n = try_decrypt_ballot(&ivk, &ballot.actions[0])?;
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
        let hash = hex::decode(TEST_ELECTION_HASH).unwrap();
        let domain = get_domain(&mut conn, 1, 1).await?;
        let (question,): (u32,) = query_as("SELECT id_question FROM questions WHERE domain = ?1")
            .bind(domain.to_repr().as_slice())
            .fetch_one(&mut *conn)
            .await?;
        let ballot = test_ballot(&mut conn, domain).await?;
        for itx in 0..2 {
            query(
                "INSERT INTO ballots(height, itx, question, data, witness)
        VALUES (1, ?1, ?2, ?3, '{}')",
            )
            .bind(itx)
            .bind(question)
            .bind(serde_json::to_string(&ballot).unwrap())
            .execute(&mut *conn)
            .await?;
        }
        tally_ballots(&Network::MainNetwork, &mut conn, TEST_ELECTION_SEED, &hash).await?;
        let (votes,): (i64,) = query_as("SELECT votes FROM results WHERE question_ref = '2.2'")
            .fetch_one(&mut *conn)
            .await?;
        assert_eq!(votes, 270_000); // 2 ballots of 135_000
        Ok(())
    }
}
