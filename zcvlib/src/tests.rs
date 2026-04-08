use std::cell::LazyCell;

use anyhow::Result;
use hex_literal::hex;
use orchard_vote::BallotData;
use pasta_curves::Fp;
use rand_core::OsRng;
use serde_json::{Value, json};
use sqlx::{SqliteConnection, pool::PoolConnection};
use zcash_protocol::consensus::Network;

use crate::{
    ZCVResult,
    ballot::encrypt_ballot_data,
    context::BFTContext,
    db::{create_schema, set_account_seed, store_election},
    pod::ElectionProps,
};

pub const TEST_SEED: &str = "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew";
pub const TEST_SEED2: &str = "purity comic seek skull unfair host point dutch drive fiction frame race hollow glow render okay add slogan upset use sick cinnamon horn lock";
pub const TEST_ELECTION_SEED: &str =
    "stool rich together paddle together pool raccoon promote attitude peasant latin concert";
pub const TEST_ELECTION_HASH: &[u8] =
    &hex!("b421701e93852a875f71fc07a9f09360028c30bb08ec90eec1a1cad536953b05");

#[allow(clippy::declare_interior_mutable_const)]
pub const TEST_ELECTION: LazyCell<Value> = LazyCell::new(|| json!({
        "pir": "",
        "end": 3169000,
        "need_sig": true,
        "name": "Test Election",
        "caption": "Test test test",
        "questions": [
            {
                "title": "Q1. What is your favorite color?",
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
            }
        ]
    }));

pub async fn test_context() -> Result<BFTContext> {
    let ctx = BFTContext::new("vote.db", "", 0, false).await?;
    Ok(ctx)
}

pub async fn test_ballot(
    conn: &mut SqliteConnection,
    domain: Fp,
    address: &str,
    memo: &[u8],
) -> ZCVResult<BallotData> {
    let ballot = encrypt_ballot_data(
        &Network::MainNetwork,
        conn,
        domain,
        0,
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
    let e = TEST_ELECTION;
    let e: ElectionProps = serde_json::from_value(e.clone()).unwrap();
    let e = e.build(TEST_ELECTION_SEED)?;
    store_election(conn, 0, "", &e, &[], &[]).await?;
    Ok(())
}
