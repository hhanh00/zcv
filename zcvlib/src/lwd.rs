use crate::{
    ZCVResult,
    db::{get_ivks, store_received_note},
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
use sqlx::{Row, SqliteConnection, query, sqlite::SqliteRow};
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
    .fetch_all(&mut *conn)
    .await?;
    let (fvk, eivk, iivk) = get_ivks(network, conn).await?;
    let ivks = [
        (0, PreparedIncomingViewingKey::new(&eivk)),
        (1, PreparedIncomingViewingKey::new(&iivk)),
    ];

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

                let act = CompactAction::from_parts(
                    Nullifier::from_bytes(&tiu!(nullifier)).unwrap(),
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
                                conn,
                                *domain,
                                &fvk,
                                &note,
                                &address,
                                true,
                                height,
                                position,
                                *id_question,
                                *scope,
                            )
                            .await?;
                        }
                    }
                }
                position += 1;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        context::Context,
        db::{create_schema, set_account_seed},
        lwd::{connect, scan_blocks},
    };
    use anyhow::Result;
    use sqlx::{Sqlite, pool::PoolConnection};
    use zcash_protocol::consensus::Network;

    pub const TEST_SEED: &str = "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew";

    async fn setup() -> Result<PoolConnection<Sqlite>> {
        let ctx = Context::new("vote.db", "").await?;
        let mut conn = ctx.connect().await?;
        create_schema(&mut conn).await?;
        set_account_seed(&mut conn, TEST_SEED, 0).await?;
        Ok(conn)
    }

    #[tokio::test]
    async fn test_scan_blocks() -> Result<()> {
        let mut conn = setup().await?;
        let mut client = connect("https://zec.rocks").await?;
        scan_blocks(
            &Network::MainNetwork,
            &mut conn,
            &mut client,
            1,
            3_168_000,
            3_169_000,
        )
        .await?;
        Ok(())
    }
}
