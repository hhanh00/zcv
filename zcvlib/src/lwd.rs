use std::collections::{HashMap, HashSet};

use crate::{
    ZCVResult, api::ProgressReporter, db::{
        derive_spending_key, get_domains, get_ivks, list_unspent_nullifiers, store_ballot_spend,
        store_election_height_position, store_received_note, store_result, store_spend,
    }, error::IntoAnyhow, rpc::{
        BlockId, BlockRange, CompactOrchardAction, PoolType,
        compact_tx_streamer_client::CompactTxStreamerClient,
    }, tiu, vote_rpc::{VoteRange, vote_streamer_client::VoteStreamerClient}
};
use ff::PrimeField;
use orchard::{
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
    note::{ExtractedNoteCommitment, Nullifier},
    note_encryption::{CompactAction, OrchardDomain},
    vote::try_decrypt_ballot,
};
use pasta_curves::Fp;
use sqlx::{Acquire, SqliteConnection, query};
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};
use tracing::info;
use zcash_note_encryption::{EphemeralKeyBytes, try_compact_note_decryption};
use zcash_protocol::consensus::Network;

pub type Client = CompactTxStreamerClient<Channel>;
pub type VoteClient = VoteStreamerClient<Channel>;

pub async fn connect(url: &str) -> ZCVResult<Client> {
    let ep = Endpoint::from_shared(url.to_string())?;
    let client = CompactTxStreamerClient::connect(ep).await?;
    Ok(client)
}

#[allow(clippy::too_many_arguments)]
pub async fn scan_blocks<PR: ProgressReporter>(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut Client,
    hash: &[u8],
    id_account: u32,
    start: u32,
    end: u32,
    pr: &PR,
) -> ZCVResult<()> {
    let mut db_tx = conn.begin().await?;
    query("DELETE FROM v_notes").execute(&mut *db_tx).await?;
    query("DELETE FROM v_spends").execute(&mut *db_tx).await?;
    let domains = get_domains(&mut db_tx, hash).await?;

    let (fvk, eivk, iivk) = get_ivks(network, &mut db_tx, id_account).await?;
    let ivks = [
        (0, PreparedIncomingViewingKey::new(&eivk)),
        (1, PreparedIncomingViewingKey::new(&iivk)),
    ];

    let mut nfs: HashSet<[u8; 32]> = HashSet::new();

    let mut blocks = client
        .get_block_range(Request::new(BlockRange {
            start: Some(BlockId {
                height: (start + 1) as u64,
                hash: vec![],
            }),
            end: Some(BlockId {
                height: end as u64,
                hash: vec![],
            }),
            pool_types: vec![PoolType::Orchard.into()],
        }))
        .await?
        .into_inner();
    let report_interval = (end - start) / 100;

    let mut position = crate::db::get_election_position(&mut db_tx, hash).await?;

    while let Some(block) = blocks.message().await? {
        let height = block.height as u32;
        if (height - start).is_multiple_of(report_interval) {
            let p = (height - start) / report_interval;
            pr.report(p);
        }

        for tx in block.vtx {
            for a in tx.actions {
                let CompactOrchardAction {
                    nullifier,
                    cmx,
                    ephemeral_key,
                    ciphertext,
                } = a;
                let nf: [u8; 32] = tiu!(nullifier);
                if nfs.contains(&nf) {
                    for (id_question, _, _) in domains.iter() {
                        store_spend(&mut db_tx, *id_question, &nf, height).await?;
                    }
                }

                let act = CompactAction::from_parts(
                    Nullifier::from_bytes(&nf).unwrap(),
                    ExtractedNoteCommitment::from_bytes(&tiu!(cmx)).unwrap(),
                    EphemeralKeyBytes(tiu!(ephemeral_key)),
                    tiu!(ciphertext),
                );

                let domain = OrchardDomain::for_compact_action(&act);
                for (scope, pivk) in ivks.iter() {
                    if let Some((note, _)) = try_compact_note_decryption(&domain, pivk, &act) {
                        info!("Found note at {} for {} zats", height, note.value().inner());

                        for (id_question, _, domain) in domains.iter() {
                            store_received_note(
                                &mut db_tx,
                                *domain,
                                id_account,
                                &fvk,
                                &note,
                                &[], // memos are not used prior to voting
                                height,
                                position,
                                *id_question,
                                *scope,
                            )
                            .await?;
                        }

                        // track new note nullifier
                        let nf = note.nullifier(&fvk).to_bytes();
                        nfs.insert(nf);
                    }
                }

                position += 1;
            }
        }
    }
    query(
        "UPDATE v_elections SET height = ?1, position = ?2
    WHERE hash = ?3",
    )
    .bind(end)
    .bind(position)
    .bind(hash)
    .execute(&mut *db_tx)
    .await?;
    db_tx.commit().await?;
    Ok(())
}

pub async fn scan_ballots(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut VoteClient,
    hash: &[u8],
    id_accounts: &[u32],
    start: u32,
    end: u32,
) -> ZCVResult<()> {
    let mut db_tx = conn.begin().await?;
    tracing::info!("scan_ballots [{start},{end}]");
    crate::db::delete_range(&mut db_tx, start, end).await?;
    let domains = get_domains(&mut db_tx, hash).await?;
    let mut ivks = vec![];
    for id_account in id_accounts {
        let (fvk, eivk, iivk) = get_ivks(network, &mut db_tx, *id_account).await?;
        ivks.push((*id_account, fvk.clone(), 0, PreparedIncomingViewingKey::new(&eivk)));
        ivks.push((*id_account, fvk.clone(), 1, PreparedIncomingViewingKey::new(&iivk)));
    }

    let mut nfs: HashMap<[u8; 32], u32> = HashMap::new();
    for id_account in id_accounts {
        for dnf in list_unspent_nullifiers(&mut db_tx, *id_account).await? {
            tracing::info!("dnf: {}", hex::encode(&dnf));
            nfs.insert(tiu!(dnf), *id_account);
        }
    }

    let mut ballots = client
        .get_vote_range(Request::new(VoteRange {
            start: (start + 1),
            end,
        }))
        .await?
        .into_inner();

    let mut position = crate::db::get_election_position(&mut db_tx, hash).await?;

    while let Some(ballot) = ballots.message().await? {
        tracing::info!("Ballot @{}", ballot.height);
        let height = ballot.height;
        let ballot = orchard::vote::Ballot::read(&*ballot.ballot).anyhow()?;
        let data = &ballot.data;
        let domain = Fp::from_repr(data.domain).unwrap();
        let id_question = domains.iter().find(|&d| d.2 == domain).unwrap().0;
        for a in data.actions.iter() {
            tracing::info!("-nf: {}", hex::encode(a.nf));
            if let Some(id_account) = nfs.get(&a.nf) {
                tracing::info!("Spend for {id_question}");
                store_ballot_spend(&mut db_tx, *id_account, id_question, &a.nf, height).await?;
            }

            for (id_account, fvk, scope, pivk) in ivks.iter() {
                if let Some((note, memo)) = try_decrypt_ballot(pivk, a.clone())? {
                    info!("Found note at {} for {} zats", height, note.value().inner());

                    store_received_note(
                        &mut db_tx,
                        domain,
                        *id_account,
                        fvk,
                        &note,
                        &memo, // memos are not used prior to voting
                        height,
                        position,
                        id_question,
                        *scope,
                    )
                    .await?;

                    // track new note nullifier
                    let nf = note.nullifier_domain(fvk, domain).to_bytes();
                    nfs.insert(nf, *id_account);
                }
            }

            position += 1;
        }
    }
    tracing::info!("height: {end}, position: {position}");
    store_election_height_position(&mut db_tx, hash, end, position).await?;
    db_tx.commit().await?;
    Ok(())
}

pub async fn decode_ballots(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut VoteClient,
    hash: &[u8],
    election_seed: &str,
    start: u32,
    end: u32,
) -> ZCVResult<()> {
    let mut db_tx = conn.begin().await?;
    let domains = get_domains(&mut db_tx, hash).await?;
    let mut vks = vec![];
    for (i, _) in domains.iter().enumerate() {
        let sk = derive_spending_key(network, election_seed, i as u32)?;
        let fvk = FullViewingKey::from(&sk);
        let ivk = fvk.to_ivk(Scope::External);
        let pivk = PreparedIncomingViewingKey::new(&ivk);
        vks.push(pivk);
    }

    let mut ballots = client
        .get_vote_range(Request::new(VoteRange {
            start: (start + 1),
            end,
        }))
        .await?
        .into_inner();

    while let Some(ballot) = ballots.message().await? {
        tracing::info!("Ballot @{}", ballot.height);
        let height = ballot.height;
        let ballot = orchard::vote::Ballot::read(&*ballot.ballot).anyhow()?;
        let data = &ballot.data;
        let domain = Fp::from_repr(data.domain).unwrap();
        let (_, idx_question, _) = domains.iter().find(|&d| d.2 == domain).unwrap();
        for a in data.actions.iter() {
            for pivk in vks.iter() {
                if let Some((note, memo)) = try_decrypt_ballot(pivk, a.clone())? {
                    info!(
                        "Found note at {} for {} zats to question {} with answer {}",
                        height,
                        note.value().inner(),
                        *idx_question,
                        hex::encode(&memo[..64])
                    );

                    store_result(&mut db_tx, *idx_question, &memo, note.value().inner()).await?;
                }
            }
        }
    }
    db_tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::tests::{get_connection, run_scan, test_setup};

    #[tokio::test]
    #[serial_test::serial]
    pub async fn test_scan() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        Ok(())
    }
}
