use std::{
    collections::{HashMap, HashSet},
    io::Write,
};

use crate::{
    ZCVResult,
    api::ProgressReporter,
    db::{
        derive_spending_key, get_domain, get_ivks, list_unspent_nullifiers, store_ballot_spend,
        store_cmx, store_election_height_position, store_nf_cmx, store_received_note, store_result,
        store_spend,
    },
    error::IntoAnyhow,
    rpc::{
        BlockId, BlockRange, CompactOrchardAction, PoolType,
        compact_tx_streamer_client::CompactTxStreamerClient,
    },
    tiu,
    vote::expand_into_ranges,
    vote_rpc::{VoteRange, vote_streamer_client::VoteStreamerClient},
};
use byteorder::{LE, WriteBytesExt};
use ff::PrimeField;
use incrementalmerkletree::frontier::Frontier;
use orchard::{
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
    note::{ExtractedNoteCommitment, Nullifier},
    note_encryption::{CompactAction, OrchardDomain},
    tree::MerkleHashOrchard,
    vote::{calculate_merkle_paths, try_decrypt_ballot},
};
use pasta_curves::Fp;
use sqlx::{Acquire, SqliteConnection, query, query_as};
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};
use tracing::info;
use zcash_encoding::Vector;
use zcash_note_encryption::{EphemeralKeyBytes, try_compact_note_decryption};
use zcash_protocol::consensus::Network;

pub type Client = CompactTxStreamerClient<Channel>;
pub type VoteClient = VoteStreamerClient<Channel>;

pub async fn connect(url: &str) -> ZCVResult<Client> {
    let ep = Endpoint::from_shared(url.to_string())?;
    let client = CompactTxStreamerClient::connect(ep).await?;
    Ok(client)
}

pub async fn initial_scan(
    client: &mut Client,
    start: u32,
    end: u32,
) -> ZCVResult<(Vec<u8>, Vec<u8>)> {
    let mut blocks = client
        .get_block_range(Request::new(BlockRange {
            start: Some(BlockId {
                height: start as u64,
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

    let mut nfs = vec![];
    let mut cmx_tree = Frontier::<MerkleHashOrchard, 32>::empty();
    while let Some(block) = blocks.message().await? {
        if block.height % 10_000 == 0 {
            tracing::info!("initial_scan: @{}", block.height);
        }
        for tx in block.vtx {
            for a in tx.actions {
                let CompactOrchardAction { nullifier, cmx, .. } = a;
                let nf = Fp::from_repr(tiu!(nullifier)).unwrap();
                nfs.push(nf);
                cmx_tree.append(MerkleHashOrchard::from_bytes(&tiu!(cmx)).unwrap());
            }
        }
    }
    nfs.sort();
    let nfs = expand_into_ranges(nfs);
    let (root, _) = calculate_merkle_paths(0, &[], &nfs);
    let nf_root = root.to_repr().to_vec();
    let (p, l, o) = cmx_tree.take().unwrap().into_parts();
    let mut cmx_tree_state = vec![];
    cmx_tree_state.write_u64::<LE>(u64::from(p)).anyhow()?;
    cmx_tree_state.write_all(&l.to_bytes()).anyhow()?;
    Vector::write(&mut cmx_tree_state, &o, |w, h| w.write_all(&h.to_bytes())).anyhow()?;

    Ok((nf_root, cmx_tree_state))
}

#[allow(clippy::too_many_arguments)]
pub async fn scan_blocks<PR: ProgressReporter>(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut Client,
    id_accounts: &[u32],
    pr: &PR,
) -> ZCVResult<()> {
    let (start, end, height): (u32, u32, u32) =
        query_as("SELECT start, end, height FROM v_elections WHERE id_election = 0")
            .fetch_one(&mut *conn)
            .await?;
    if end <= height {
        return Ok(());
    }
    tracing::info!("scan_blocks [{start},{end}]");

    let mut db_tx = conn.begin().await?;
    query("DELETE FROM v_notes").execute(&mut *db_tx).await?;
    query("DELETE FROM v_spends").execute(&mut *db_tx).await?;
    query("DELETE FROM vc_nfs").execute(&mut *db_tx).await?;
    query("DELETE FROM vc_cmxs").execute(&mut *db_tx).await?;
    let (domain, _) = get_domain(&mut db_tx).await?;

    let mut ivks = vec![];
    for id_account in id_accounts {
        let (fvk, eivk, iivk) = get_ivks(network, &mut db_tx, *id_account).await?;
        ivks.push((
            *id_account,
            0,
            fvk.clone(),
            PreparedIncomingViewingKey::new(&eivk),
        ));
        ivks.push((
            *id_account,
            1,
            fvk.clone(),
            PreparedIncomingViewingKey::new(&iivk),
        ));
    }

    let mut nfs: HashSet<[u8; 32]> = HashSet::new();

    let mut blocks = client
        .get_block_range(Request::new(BlockRange {
            start: Some(BlockId {
                height: start as u64,
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

    let mut position = crate::db::get_election_position(&mut db_tx).await?;

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
                store_nf_cmx(&mut db_tx, &nullifier, &cmx).await?;

                let nf: [u8; 32] = tiu!(nullifier);
                if nfs.contains(&nf) {
                    store_spend(&mut db_tx, &nf, height).await?;
                }

                let act = CompactAction::from_parts(
                    Nullifier::from_bytes(&nf).unwrap(),
                    ExtractedNoteCommitment::from_bytes(&tiu!(cmx)).unwrap(),
                    EphemeralKeyBytes(tiu!(ephemeral_key)),
                    tiu!(ciphertext),
                );

                let orchard_domain = OrchardDomain::for_compact_action(&act);
                for (id_account, scope, fvk, pivk) in ivks.iter() {
                    if let Some((note, _)) =
                        try_compact_note_decryption(&orchard_domain, pivk, &act)
                    {
                        info!("Found note at {} for {} zats", height, note.value().inner());

                        store_received_note(
                            &mut db_tx,
                            domain,
                            *id_account,
                            fvk,
                            &note,
                            &[], // memos are not used prior to voting
                            height,
                            position,
                            *scope,
                        )
                        .await?;

                        // track new note nullifier
                        let nf = note.nullifier(fvk).to_bytes();
                        nfs.insert(nf);
                    }
                }

                position += 1;
            }
        }
    }
    query(
        "UPDATE v_elections SET height = ?1, position = ?2
    WHERE id_election = 0",
    )
    .bind(end)
    .bind(position)
    .execute(&mut *db_tx)
    .await?;
    db_tx.commit().await?;
    Ok(())
}

pub async fn scan_ballots(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut VoteClient,
    id_accounts: &[u32],
    start: u32,
    end: u32,
) -> ZCVResult<()> {
    tracing::info!("scan_ballots [{start},{end}]");
    if start > end {
        tracing::info!("Skipping scan_ballots");
        return Ok(());
    }
    let mut db_tx = conn.begin().await?;
    crate::db::delete_range(&mut db_tx, start, end).await?;
    let mut ivks = vec![];
    for id_account in id_accounts {
        let (fvk, eivk, iivk) = get_ivks(network, &mut db_tx, *id_account).await?;
        ivks.push((
            *id_account,
            fvk.clone(),
            0,
            PreparedIncomingViewingKey::new(&eivk),
        ));
        ivks.push((
            *id_account,
            fvk.clone(),
            1,
            PreparedIncomingViewingKey::new(&iivk),
        ));
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

    let mut position = crate::db::get_election_position(&mut db_tx).await?;

    while let Some(ballot) = ballots.message().await? {
        let height = ballot.height;
        let ballot = orchard::vote::Ballot::read(&*ballot.ballot).anyhow()?;
        let data = &ballot.data;
        let domain = Fp::from_repr(data.domain).unwrap();
        for a in data.actions.iter() {
            // do not store nf since we are on the voting chain
            store_cmx(&mut db_tx, &a.cmx).await?;
            if let Some(id_account) = nfs.get(&a.nf) {
                store_ballot_spend(&mut db_tx, *id_account, &a.nf, height).await?;
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
    store_election_height_position(&mut db_tx, end, position).await?;
    db_tx.commit().await?;
    Ok(())
}

pub async fn decode_ballots(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut VoteClient,
    election_seed: &str,
    start: u32,
    end: u32,
) -> ZCVResult<()> {
    let mut db_tx = conn.begin().await?;
    let sk = derive_spending_key(network, election_seed, 0)?;
    let fvk = FullViewingKey::from(&sk);
    let ivk = fvk.to_ivk(Scope::External);
    let pivk = PreparedIncomingViewingKey::new(&ivk);

    query("DELETE FROM v_results")
    .execute(&mut *db_tx)
    .await?;

    let mut ballots = client
        .get_vote_range(Request::new(VoteRange {
            start: (start + 1),
            end,
        }))
        .await?
        .into_inner();

    while let Some(ballot) = ballots.message().await? {
        let height = ballot.height;
        let ballot = orchard::vote::Ballot::read(&*ballot.ballot).anyhow()?;
        let data = &ballot.data;
        for a in data.actions.iter() {
            if let Some((note, memo)) = try_decrypt_ballot(&pivk, a.clone())? {
                info!(
                    "Found note at {} for {} zats with answer {}",
                    height,
                    note.value().inner(),
                    hex::encode(&memo[..64])
                );

                store_result(&mut db_tx, &memo, note.value().inner()).await?;
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
