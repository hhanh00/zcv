use ff::PrimeField;
use orchard::{
    Address,
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
    vote::{BallotAnchors, BallotData, dummy_vote, encrypt_ballot_action, try_decrypt_ballot},
};
use pasta_curves::Fp;
use rand::seq::SliceRandom;
use rand_core::{CryptoRng, RngCore};
use sqlx::SqliteConnection;
use zcash_protocol::consensus::Network;

use crate::{
    ZCVError, ZCVResult,
    balance::list_unspent_notes,
    db::{get_ivks, store_received_note},
    error::IntoAnyhow,
    pod::UTXO,
    tiu,
};

#[allow(clippy::too_many_arguments)]
pub async fn encrypt_ballot_data<R: CryptoRng + RngCore>(
    network: &Network,
    conn: &mut SqliteConnection,
    domain: Fp,
    id_account: u32,
    address: &str,
    memo: &[u8],
    amount: u64,
    mut rng: R,
) -> ZCVResult<BallotData> {
    let mut utxos = list_unspent_notes(conn, domain, id_account).await?;
    utxos.shuffle(&mut rng);
    let mut sum = 0;
    let utxos: Vec<_> = utxos
        .into_iter()
        .take_while(|u| {
            let r = sum < amount;
            sum += u.value;
            r
        })
        .collect();
    let amount_utxo = utxos.iter().map(|u| u.value).sum::<u64>();
    if amount_utxo < amount {
        return Err(ZCVError::NotEnoughVotes);
    }
    encrypt_ballot_data_with_spends(
        network,
        conn,
        domain,
        id_account,
        address,
        memo,
        amount,
        utxos,
        amount_utxo,
        rng,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn encrypt_ballot_data_with_spends<R: CryptoRng + RngCore>(
    network: &Network,
    conn: &mut SqliteConnection,
    domain: Fp,
    id_account: u32,
    address: &str,
    memo: &[u8],
    amount: u64,
    mut spends: Vec<UTXO>,
    amount_spent: u64,
    mut rng: R,
) -> ZCVResult<BallotData> {
    let (fvk, _, ivk) = get_ivks(network, conn, id_account).await?;
    let change_address = ivk.address_at(0u64);
    let (_, recipient) = bech32::decode(address).anyhow()?;
    let recipient = Address::from_raw_address_bytes(&tiu!(recipient)).unwrap();
    if spends.len() < 2 {
        // pad
        let len = spends.len();
        for _ in 0..(2 - len) {
            let (_, fvk, note) = dummy_vote(&mut rng);
            let nf = note.nullifier(&fvk);
            let dnf = note.nullifier_domain(&fvk, domain);
            let recipient = note.recipient();
            spends.push(UTXO {
                height: 0,
                scope: 0,
                position: 0,
                nf: nf.to_bytes().to_vec(),
                dnf: dnf.to_bytes().to_vec(),
                rho: note.rho().to_bytes().to_vec(),
                diversifier: recipient.diversifier().as_array().to_vec(),
                rseed: note.rseed().as_bytes().to_vec(),
                value: 0,
            });
        }
    }
    assert!(spends.len() >= 2);
    // We do not check that amount_spent == sum(spends.amount)
    // because this is an internal function and the caller
    // should have calculated the amount_spent
    // This function is used when testing ballot outputs
    if amount_spent < amount {
        return Err(ZCVError::NotEnoughVotes);
    }

    let mut actions = vec![];
    for (i, spend) in spends.into_iter().enumerate() {
        let spend = spend.to_note(&fvk);
        let (output_amount, destination) = if i == 0 {
            (amount, recipient)
        } else if i == 1 {
            let change = amount_spent - amount;
            (change, change_address)
        } else {
            (0, recipient)
        };
        let (action, _, _) = encrypt_ballot_action(
            domain,
            fvk.clone(),
            &spend,
            destination,
            output_amount,
            memo,
            &mut rng,
        )?;
        actions.push(action);
    }
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
    id_account: u32,
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
                id_account,
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

// pub const MAX_CHOICES: usize = 32;
// pub const MAX_QUESTIONS: usize = 64;

// fn plurality(
//     question: u32,
//     amount: u64,
//     memo: [u8; 512],
//     counts: &mut [u64; MAX_CHOICES * MAX_QUESTIONS],
// ) -> ZCVResult<()> {
//     // each byte in the memo is a "vote" for the choice at
//     // that offset
//     // we do not use the complete memo in this type of voting
//     #[allow(clippy::needless_range_loop)]
//     for i in 0..MAX_CHOICES {
//         let idx = question as usize * MAX_CHOICES + i;
//         if memo[i] != 0 {
//             counts[idx] += amount;
//         }
//     }
//     Ok(())
// }

// pub async fn tally_plurality_election(
//     network: &Network,
//     conn: &mut SqliteConnection,
//     election_seed: &str,
//     hash: &[u8],
// ) -> ZCVResult<()> {
//     let counts = tally_ballots(
//         network,
//         conn,
//         election_seed,
//         hash,
//         [0u64; MAX_CHOICES * MAX_QUESTIONS],
//         plurality,
//     )
//     .await?;
//     for (i, c) in counts.iter().enumerate() {
//         if *c == 0 {
//             continue;
//         }
//         let idx_question = i / MAX_CHOICES;
//         let idx_votes = i % MAX_CHOICES;
//         query(
//             "INSERT INTO results(question, answer, votes)
//         VALUES (?1, ?2, ?3)
//         ON CONFLICT DO UPDATE SET
//         votes = votes + excluded.votes",
//         )
//         .bind(idx_question as u32)
//         .bind(idx_votes as u32)
//         .bind(*c as i64)
//         .execute(&mut *conn)
//         .await?;
//     }
//     Ok(())
// }

// pub async fn tally_ballots<R>(
//     network: &Network,
//     conn: &mut SqliteConnection,
//     election_seed: &str,
//     hash: &[u8],
//     mut result: R,
//     memo_handler: impl Fn(u32, u64, [u8; 512], &mut R) -> ZCVResult<()>,
// ) -> ZCVResult<R> {
//     let domains: Vec<(u32, String)> = query_as(
//         "SELECT idx, address FROM questions q
//     JOIN elections e ON q.election = e.id_election
//     WHERE e.hash = ?1 ORDER BY idx",
//     )
//     .bind(hash)
//     .fetch_all(&mut *conn)
//     .await?;
//     let mut ivks: HashMap<u32, PreparedIncomingViewingKey> = HashMap::new();
//     for (idx, address) in domains {
//         let spk = derive_spending_key(network, election_seed, idx)?;
//         let fvk = FullViewingKey::from(&spk);
//         let ivk = fvk.to_ivk(Scope::External);
//         let pivk = PreparedIncomingViewingKey::new(&ivk);
//         let address2 = fvk.address_at(0u64, Scope::External);
//         let address2 = bech32::encode::<Bech32m>(
//             Hrp::parse(ZCV_HRP).unwrap(),
//             &address2.to_raw_address_bytes(),
//         )
//         .anyhow()?;
//         assert_eq!(address, address2);
//         ivks.insert(idx, pivk);
//     }

//     fetch_ballots(conn, async |question, ballot_data| {
//         let pivk = &ivks[&question];
//         for action in ballot_data.actions.into_iter() {
//             if let Some((note, memo)) = try_decrypt_ballot(pivk, action)? {
//                 let votes = note.value().inner();
//                 memo_handler(question, votes, memo, &mut result)?;
//             }
//         }
//         Ok(())
//     })
//     .await?;

//     Ok(result)
// }

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use anyhow::Result;
    use base64::Engine;
    use bech32::{Bech32m, Hrp};
    use orchard::{
        keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
        vote::{Ballot, BallotWitnesses, try_decrypt_ballot},
    };
    use rand_core::OsRng;
    use zcash_protocol::consensus::Network;

    use crate::{
        ZCVResult,
        ballot::{encrypt_ballot_data, encrypt_ballot_data_with_spends},
        db::{derive_spending_key, get_domain},
        error::IntoAnyhow,
        pod::ZCV_HRP,
        tests::{
            TEST_ELECTION_HASH, TEST_ELECTION_SEED, TEST_SEED, get_connection, run_scan,
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
        tracing::info!("Sending ballot to {}", address);
        let ballot = encrypt_ballot_data(
            &Network::MainNetwork,
            &mut conn,
            domain,
            0,
            &address,
            &[], /* answer */
            100000,
            OsRng,
        )
        .await?;
        let spk = derive_spending_key(&Network::MainNetwork, TEST_ELECTION_SEED, 2).anyhow()?;
        let fvk = FullViewingKey::from(&spk);
        let ivk = PreparedIncomingViewingKey::new(&fvk.to_ivk(Scope::External));
        let n = try_decrypt_ballot(&ivk, ballot.actions[0].clone())?;
        assert!(n.is_some());
        Ok(())
    }

    // #[tokio::test]
    // #[serial_test::serial]
    // async fn test_tally_ballots() -> Result<()> {
    //     let mut conn = get_connection().await?;
    //     test_setup(&mut conn).await?;
    //     run_scan(&mut conn).await?;
    //     query("DELETE FROM ballots").execute(&mut *conn).await?;
    //     query("DELETE FROM results").execute(&mut *conn).await?;
    //     let mut tx = conn.begin().await?;
    //     let (domain, address) = get_domain(&mut tx, TEST_ELECTION_HASH, 1).await?;
    //     let question = get_question(&mut tx, domain).await?;
    //     query("UPDATE notes SET value = value * 100000000")
    //         .execute(&mut *tx)
    //         .await?;
    //     query("UPDATE spends SET value = value * 100000000")
    //         .execute(&mut *tx)
    //         .await?;
    //     let ballot = test_ballot(&mut tx, domain, &address, &[0, 0, 1, 0]).await?;
    //     let mut ballot_bytes = vec![];
    //     ballot.write(&mut ballot_bytes)?;
    //     for itx in 0..2 {
    //         query(
    //             "INSERT INTO ballots(height, itx, question, data, witnesses)
    //     VALUES (1, ?1, ?2, ?3, '')",
    //         )
    //         .bind(itx)
    //         .bind(question.index as u32)
    //         .bind(&ballot_bytes)
    //         .execute(&mut *tx)
    //         .await?;
    //     }
    //     tally_plurality_election(
    //         &Network::MainNetwork,
    //         &mut tx,
    //         TEST_ELECTION_SEED,
    //         TEST_ELECTION_HASH,
    //     )
    //     .await?;
    //     let (votes,): (i64,) =
    //         query_as("SELECT votes FROM results WHERE question = 1 and answer = 2")
    //             .fetch_one(&mut *tx)
    //             .await?;
    //     assert_eq!(votes, 27_000_000_000_000); // 2 ballots of 135_000
    //     query("UPDATE notes SET value = value / 100000000")
    //         .execute(&mut *tx)
    //         .await?;
    //     query("UPDATE spends SET value = value / 100000000")
    //         .execute(&mut *tx)
    //         .await?;

    //     tx.commit().await?;
    //     Ok(())
    // }

    #[tokio::test]
    async fn make_ballot_bin() -> ZCVResult<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        let (domain, address) = get_domain(&mut conn, TEST_ELECTION_HASH, 1).await?;
        let ballot_data = encrypt_ballot_data_with_spends(
            &Network::MainNetwork,
            &mut conn,
            domain,
            0,
            &address,
            &[1, 1, 1, 1],
            2_400_000_000_000u64,
            vec![],
            8_000_000_000_000u64,
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
        assert_eq!(ballot_bytes.len(), 1647);
        tracing::info!(
            "{}",
            base64::engine::general_purpose::STANDARD.encode(&ballot_bytes)
        );
        Ok(())
    }

    #[tokio::test]
    pub async fn test_ballot_scripts() -> Result<()> {
        let mut script_file = File::create("add_ballots.sh")?;
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        let sk = derive_spending_key(&Network::MainNetwork, TEST_SEED, 0)?;
        let fvk = FullViewingKey::from(&sk);
        let address = fvk.to_ivk(Scope::External).address_at(0u64);
        let hrp = Hrp::parse(ZCV_HRP).anyhow()?;
        let address = bech32::encode::<Bech32m>(hrp, &address.to_raw_address_bytes())?;
        let (domain, _) = get_domain(&mut conn, TEST_ELECTION_HASH, 1).await?;
        let ballot_data = encrypt_ballot_data_with_spends(
            &Network::MainNetwork,
            &mut conn,
            domain,
            0,
            &address,
            &[],
            8_000_000_000_000u64,
            vec![],
            8_000_000_000_000u64,
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
        let ballot = to_base64(ballot)?;
        writeln!(script_file, "# mint")?;
        writeln!(script_file,
            "grpcurl -d '{{\"ballot\": \"{ballot}\"}}' --proto zcvlib/protos/vote.proto --plaintext localhost:9010 cash.z.vote.sdk.rpc.VoteStreamer/SubmitVote"
        )?;
        Ok(())
    }

    fn to_base64(ballot: Ballot) -> Result<String> {
        let mut ballot_bytes = vec![];
        ballot.write(&mut ballot_bytes).anyhow()?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&ballot_bytes))
    }
}
