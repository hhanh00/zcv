use anyhow::Result;
use orchard::vote::BallotData;
use pasta_curves::Fp;
use rand_core::OsRng;
use serde_json::json;
use sqlx::{SqliteConnection, pool::PoolConnection, query_as};
use zcash_protocol::consensus::Network;
use hex_literal::hex;

use crate::{
    ZCVResult, ballot::encrypt_ballot_data, context::Context, db::{create_schema, set_account_seed}, lwd::{connect, scan_blocks}, pod::ElectionProps
};

pub const TEST_SEED: &str = "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew";
pub const TEST_ELECTION_SEED: &str =
    "stool rich together paddle together pool raccoon promote attitude peasant latin concert";
pub const TEST_ELECTION_HASH: &[u8] =
    &hex!("3A1ECEBCB20EABF35941C08717ECDAE080CA3F20279BD2BD8C757E74C82F8359");

pub async fn test_context() -> Result<Context> {
    let ctx = Context::new("vote.db", "", 0).await?;
    Ok(ctx)
}

pub async fn test_ballot(conn: &mut SqliteConnection, domain: Fp, address: &str, memo: &[u8]) -> ZCVResult<BallotData> {
    let ballot = encrypt_ballot_data(
        &Network::MainNetwork,
        conn,
        domain,
        address,
        memo,
        13_500_000_000_000,
        OsRng,
    )
    .await?;
    Ok(ballot)
}

pub async fn get_connection() -> Result<PoolConnection<sqlx::Sqlite>> {
    let ctx = test_context().await?;
    let mut conn = ctx.connect().await?;
    create_schema(&mut conn).await?;
    Ok(conn)
}

pub async fn test_setup(conn: &mut SqliteConnection) -> Result<()> {
    set_account_seed(conn, 0, TEST_SEED, 0).await?;
    set_account_seed(conn, 1, TEST_ELECTION_SEED, 0).await?;
    let e = json!({
        "secret_seed": TEST_ELECTION_SEED,
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
    let (c,): (u32,) = query_as("SELECT COUNT(*) FROM notes")
        .fetch_one(&mut *conn)
        .await?;
    if c != 0 {
        return Ok(());
    }

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
    // Sleep to give some time for the scan to commit
    // the utxos to the db
    std::thread::sleep(std::time::Duration::from_secs(1));
    Ok(())
}
