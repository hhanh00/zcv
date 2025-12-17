use std::collections::HashSet;

use crate::{
    ZCVResult,
    db::{get_ivks, store_received_note, store_spend},
    rpc::{
        BlockId, BlockRange, CompactOrchardAction, PoolType,
        compact_tx_streamer_client::CompactTxStreamerClient,
    },
    tiu,
};
use ff::PrimeField;
use orchard::{
    keys::PreparedIncomingViewingKey,
    note::{ExtractedNoteCommitment, Nullifier},
    note_encryption::{CompactAction, OrchardDomain},
};
use pasta_curves::Fp;
use sqlx::{Acquire, Row, SqliteConnection, query, sqlite::SqliteRow};
use tonic::{
    Request,
    transport::{Channel, Endpoint},
};
use zcash_note_encryption::{EphemeralKeyBytes, try_compact_note_decryption};
use zcash_protocol::consensus::Network;

pub type Client = CompactTxStreamerClient<Channel>;

pub async fn connect(url: &str) -> ZCVResult<Client> {
    let ep = Endpoint::from_shared(url.to_string())?;
    let client = CompactTxStreamerClient::connect(ep).await?;
    Ok(client)
}

pub async fn scan_blocks(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut Client,
    id_election: u32,
    start: u32,
    end: u32,
) -> ZCVResult<()> {
    let mut db_tx = conn.begin().await?;
    query("DELETE FROM notes").execute(&mut *db_tx).await?;
    query("DELETE FROM spends").execute(&mut *db_tx).await?;

    let domains = query(
        "SELECT id_question, domain FROM questions
        WHERE election = ?1 ORDER BY idx",
    )
    .bind(id_election)
    .map(|r: SqliteRow| {
        let idx: u32 = r.get(0);
        let d: Vec<u8> = r.get(1);
        let domain = Fp::from_repr(tiu!(d)).unwrap();
        (idx, domain)
    })
    .fetch_all(&mut *db_tx)
    .await?;
    println!("{} questions", domains.len());

    let (fvk, eivk, iivk) = get_ivks(network, &mut db_tx).await?;
    let ivks = [
        (0, PreparedIncomingViewingKey::new(&eivk)),
        (1, PreparedIncomingViewingKey::new(&iivk)),
    ];
    println!("{:?}", fvk);

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

    let mut position = 0u32;

    while let Some(block) = blocks.message().await? {
        let height = block.height as u32;
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
                    for (id_question, _) in domains.iter() {
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
                    if let Some((note, address)) = try_compact_note_decryption(&domain, pivk, &act)
                    {
                        println!("Found note at {} for {} zats", height, note.value().inner());

                        for (id_question, domain) in domains.iter() {
                            store_received_note(
                                &mut db_tx,
                                *domain,
                                &fvk,
                                &note,
                                &address,
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
    db_tx.commit().await?;
    Ok(())
}
