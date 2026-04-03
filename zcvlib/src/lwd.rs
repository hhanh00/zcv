use std::collections::HashMap;

use crate::{
    ZCVResult,
    db::{
        derive_spending_key, frontier_to_bytes, get_election_frontier, get_ivks,
        list_unspent_nullifiers, store_ballot_spend, store_cmx, store_election_frontier,
        store_election_height, store_received_note, store_result,
    },
    error::IntoAnyhow,
    rpc::{BlockId, compact_tx_streamer_client::CompactTxStreamerClient},
    tiu,
    vote_rpc::{VoteRange, vote_streamer_client::VoteStreamerClient},
};
use ff::PrimeField;
use orchard::{
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
    tree::MerkleHashOrchard,
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
use zcash_trees::warp::legacy::CommitmentTreeFrontier;

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
    let orchard_tree_state = hex::decode(tree_state.orchard_tree)
    .anyhow()?;
    let client = pir_client::PirClient::connect(pir_url).await?;
    let nf_root = client.root29().to_repr().to_vec();

    Ok((nf_root, orchard_tree_state))
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

    let cmx_tree_bytes = get_election_frontier(&mut db_tx).await?;
    let mut cmx_tree = if cmx_tree_bytes.is_empty() {
        incrementalmerkletree::frontier::Frontier::<MerkleHashOrchard, 32>::empty()
    } else {
        CommitmentTreeFrontier::read(cmx_tree_bytes.as_slice())
            .anyhow()?
            .to_orchard_frontier()
    };
    let mut position = cmx_tree.tree_size() as u32;

    while let Some(ballot) = ballots.message().await? {
        let height = ballot.height;
        let ballot = orchard::vote::Ballot::read(&*ballot.ballot).anyhow()?;
        let data = &ballot.data;
        let domain = Fp::from_repr(data.domain).unwrap();
        for a in data.actions.iter() {
            // do not store nf since we are on the voting chain
            store_cmx(&mut db_tx, &a.cmx).await?;
            let cmx = MerkleHashOrchard::from_bytes(&a.cmx).unwrap();
            cmx_tree.append(cmx);
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

                    // track new note nullifier
                    let nf = note.nullifier_domain(fvk, domain).to_bytes();
                    nfs.insert(nf, *id_account);

                    // TODO Calculate new nf and cmx proofs
                    // Store in v_witnesses
                }
            }
            position += 1;
        }
    }
    tracing::info!("height: {end}, position: {}", cmx_tree.tree_size());
    store_election_height(&mut db_tx, end).await?;
    let cmx_tree_bytes = frontier_to_bytes(&cmx_tree)?;
    store_election_frontier(&mut db_tx, &cmx_tree_bytes).await?;
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

