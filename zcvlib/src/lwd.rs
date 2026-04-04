use std::collections::HashMap;

use crate::{
    ZCVResult,
    db::{
        derive_spending_key, get_election_frontier, get_ivks,
        list_election_witnesses, list_unspent_nullifiers,
        store_ballot_spend, store_election_frontier, store_election_height,
        store_election_witness, store_received_note, store_result,
    },
    error::IntoAnyhow,
    rpc::{BlockId, compact_tx_streamer_client::CompactTxStreamerClient},
    tiu,
    vote_rpc::{VoteRange, vote_streamer_client::VoteStreamerClient},
};
use bincode::config::legacy;
use ff::PrimeField;
use orchard::{
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
    vote::try_decrypt_ballot,
};
use pasta_curves::Fp;
use sqlx::{Acquire, SqliteConnection, query};
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};
use tracing::info;
use zcash_protocol::consensus::Network;
use zcash_trees::warp::{Edge, Hasher, Witness, hasher::OrchardHasher};

pub type Client = CompactTxStreamerClient<Channel>;
pub type VoteClient = VoteStreamerClient<Channel>;

pub async fn connect(url: &str) -> ZCVResult<Client> {
    let ep = Endpoint::from_shared(url.to_string())?;
    let client = CompactTxStreamerClient::connect(ep).await?;
    Ok(client)
}

pub async fn fetch_roots(lwd_url: &str, pir_url: &str, end: u32) -> ZCVResult<(Vec<u8>, Vec<u8>)> {
    let mut lwd_client = connect(lwd_url).await?;
    let tree_state = lwd_client
        .get_tree_state(Request::new(BlockId {
            height: end as u64,
            hash: vec![],
        }))
        .await?
        .into_inner();
    let orchard_tree_state = hex::decode(tree_state.orchard_tree).anyhow()?;
    let client = pir_client::PirClient::connect(pir_url).await?;
    let nf_root = client.root29().to_repr().to_vec();

    Ok((nf_root, orchard_tree_state))
}

pub async fn scan_ballots(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut VoteClient,
    id_account: u32,
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
    let (fvk, eivk, iivk) = get_ivks(network, &mut db_tx, id_account).await?;
    ivks.push((
        id_account,
        fvk.clone(),
        0,
        PreparedIncomingViewingKey::new(&eivk),
    ));
    ivks.push((
        id_account,
        fvk.clone(),
        1,
        PreparedIncomingViewingKey::new(&iivk),
    ));

    let mut nfs: HashMap<[u8; 32], u32> = HashMap::new();
    for dnf in list_unspent_nullifiers(&mut db_tx, id_account).await? {
        tracing::info!("dnf: {}", hex::encode(&dnf));
        nfs.insert(tiu!(dnf), id_account);
    }

    let mut ballots = client
        .get_vote_range(Request::new(VoteRange {
            start: (start + 1),
            end,
        }))
        .await?
        .into_inner();

    let hasher = OrchardHasher::default();
    tracing::info!("get_election_frontier");
    let cmx_tree_bytes = get_election_frontier(&mut db_tx).await?;
    let mut edge = Edge::read(cmx_tree_bytes.as_slice()).anyhow()?;
    let mut position = edge.size() as u32;
    let initial_position = position;

    tracing::info!("ballot loop");
    let mut new_notes = vec![];
    let mut cmxs = vec![];
    while let Some(ballot) = ballots.message().await? {
        let height = ballot.height;
        let ballot = orchard::vote::Ballot::read(&*ballot.ballot).anyhow()?;
        let data = &ballot.data;
        let domain = Fp::from_repr(data.domain).unwrap();
        for a in data.actions.iter() {
            cmxs.push(Some(a.cmx));

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
                        None,
                        &note,
                        &memo, // memos are not used prior to voting
                        height,
                        position,
                        *scope,
                    )
                    .await?;

                    new_notes.push((position, note, Witness::default()));

                    // track new note nullifier
                    let nf = note.nullifier_domain(fvk, domain).to_bytes();
                    nfs.insert(nf, *id_account);
                }
            }
            position += 1;
        }
    }

    let mut old_notes = list_election_witnesses(&mut db_tx, &fvk, start).await?;

    for depth in 0..zcash_trees::warp::MERKLE_DEPTH as usize {
        let mut position = initial_position >> depth;
        if position % 2 == 1 {
            cmxs.insert(0, Some(edge.0[depth].unwrap()));
            position -= 1;
        }

        for (npos, _n, w) in new_notes.iter_mut() {
            let note_pos = *npos >> depth;
            let nidx = (note_pos - position) as usize;

            if depth == 0 {
                w.position = *npos;
                w.value = cmxs[nidx].unwrap();
            }

            if nidx.is_multiple_of(2) {
                if nidx + 1 < cmxs.len() {
                    assert!(
                        cmxs[nidx + 1].is_some(),
                        "{} {} {}",
                        depth,
                        w.position,
                        nidx
                    );
                    w.ommers.0[depth] = cmxs[nidx + 1];
                } else {
                    w.ommers.0[depth] = None;
                }
            } else {
                assert!(
                    cmxs[nidx - 1].is_some(),
                    "{} {} {}",
                    depth,
                    w.position,
                    nidx
                );
                w.ommers.0[depth] = cmxs[nidx - 1];
            }
        }

        let len = cmxs.len();
        if len >= 2 {
            for (_, _, w) in old_notes.iter_mut() {
                if w.ommers.0[depth].is_none() {
                    assert!(cmxs[1].is_some());
                    w.ommers.0[depth] = cmxs[1];
                }
            }
        }

        if len % 2 == 1 {
            edge.0[depth] = cmxs[len - 1];
        } else {
            edge.0[depth] = None;
        }

        let pairs = len / 2;
        let mut cmxs2 = hasher.parallel_combine_opt(depth as u8, &cmxs, pairs);
        std::mem::swap(&mut cmxs, &mut cmxs2);
    }

    tracing::info!("root = {}", hex::encode(edge.root(&hasher)));
    let edge_auth_path = edge.to_auth_path(&hasher);
    // store updated witnesses (old and new)
    for (id_note, w) in old_notes.iter().map(|(id, _, w)| (Some(*id), w))
        .chain(new_notes.iter().map(|(_, _, w)| (None, w)))
    {
        tracing::info!("w root = {}", hex::encode(w.root(&edge_auth_path.0, &hasher)));
        let w_bytes = bincode::encode_to_vec(w, legacy()).anyhow()?;
        store_election_witness(&mut db_tx, id_note, &w_bytes).await?;
    }

    tracing::info!("height: {end}, position: {}", edge.size());
    store_election_height(&mut db_tx, end).await?;
    store_election_frontier(&mut db_tx, &edge).await?;
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

    query("DELETE FROM v_results").execute(&mut *db_tx).await?;

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
