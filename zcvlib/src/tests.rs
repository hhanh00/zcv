use anyhow::Result;
use serde_json::json;
use sqlx::{SqliteConnection, pool::PoolConnection};
use zcash_protocol::consensus::Network;

use crate::{
    context::Context, db::{create_schema, set_account_seed}, lwd::{connect, scan_blocks}, pod::ElectionProps
};

pub const TEST_SEED: &str = "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew";

pub async fn get_connection() -> Result<PoolConnection<sqlx::Sqlite>> {
    let ctx = Context::new("vote.db", "").await?;
    let mut conn = ctx.connect().await?;
    create_schema(&mut conn).await?;
    Ok(conn)
}

pub async fn test_setup(conn: &mut SqliteConnection) -> Result<()> {
    set_account_seed(conn, TEST_SEED, 0).await?;
    let e = json!({
        "secret_seed": TEST_SEED,
        "start": 3155000,
        "end": 3169000,
        "need_sig": true,
        "name": "Test Election",
        "questions": [
            {
                "title": "Q1. What is your favorite color?",
                "subtitle": "",
                "answers": ["Red", "Green", "Blue"]
            },
            {
                "title": "Q2. Is the earth flat?",
                "subtitle": "",
                "answers": ["Yes", "No"]
            },
            {
                "title": "Q3. Do you like pizza?",
                "subtitle": "",
                "answers": ["Yes", "No"]
            },
        ]
    });
    let e: ElectionProps = serde_json::from_value(e).unwrap();
    let e = e.build()?;
    e.store(conn).await?;
    Ok(())
}

pub async fn run_scan(conn: &mut SqliteConnection) -> Result<()> {
    let mut client = connect("https://zec.rocks").await?;
    scan_blocks(
        &Network::MainNetwork,
        conn,
        &mut client,
        1,
        3_168_000,
        3_169_000,
    )
    .await?;
    Ok(())
}
