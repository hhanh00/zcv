use std::io::Write;

use anyhow::Result;
use sqlx::SqliteConnection;
use tonic::Request;
use tracing::info;

use crate::{
    rpc::{BlockId, BlockRange},
    Client, Hash, APPSTATE,
};

pub async fn get_block_range(start: u32, end: u32) -> Result<()> {
    let app = APPSTATE.lock().await;
    let mut db = app.connect().await?;
    let mut client = app.client().await?;

    get_block_range_impl(&mut db, &mut client, start, end).await
}

async fn get_block_range_impl(
    connection: &mut SqliteConnection,
    client: &mut Client,
    start: u32,
    end: u32,
) -> Result<()> {
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
            spam_filter_threshold: 0,
        }))
        .await?
        .into_inner();

    let mut buffer = Vec::new();
    buffer.reserve(2048);
    while let Some(block) = blocks.message().await? {
        buffer.clear();
        let height = block.height as u32;
        info!("{height}");
        for tx in block.vtx {
            for a in tx.actions {
                buffer.write_all(&a.nullifier)?;
                buffer.write_all(&a.cmx)?;
            }
        }
        sqlx::query(
            "INSERT OR REPLACE INTO blocks(height, data) VALUES (?1, ?2)")
            .bind(height)
            .bind(&buffer)
            .execute(&mut *connection)
            .await?;
    }

    Ok(())
}

pub struct TxData {
    pub nf: Hash,
    pub cmx: Hash,
}
