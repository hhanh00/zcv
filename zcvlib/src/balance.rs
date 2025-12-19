use sqlx::{Row, SqliteConnection, query, sqlite::SqliteRow};

use crate::{ZCVResult, pod::UTXO};

pub async fn list_unspent_notes(
    conn: &mut SqliteConnection,
    id_question: u32,
) -> ZCVResult<Vec<UTXO>> {
    let utxos = query(
        "SELECT scope, position, nf, dnf, rho, diversifier, rseed, n.value
        FROM notes n LEFT JOIN spends s ON n.id_note = s.id_note
        WHERE s.id_note IS NULL AND n.question = ?1",
    )
    .bind(id_question)
    .map(|r: SqliteRow| {
        let scope: u32 = r.get(0);
        let position: u32 = r.get(1);
        let nf: Vec<u8> = r.get(2);
        let dnf: Vec<u8> = r.get(3);
        let rho: Vec<u8> = r.get(4);
        let diversifier: Vec<u8> = r.get(5);
        let rseed: Vec<u8> = r.get(6);
        let value: u64 = r.get(7);
        UTXO {
            scope,
            position,
            nf,
            dnf,
            rho,
            diversifier,
            rseed,
            value,
        }
    })
    .fetch_all(conn)
    .await?;
    Ok(utxos)
}

pub async fn get_balance(conn: &mut SqliteConnection, id_question: u32) -> ZCVResult<u64> {
    let utxos = list_unspent_notes(conn, id_question).await?;
    let balance = utxos.iter().map(|utxo| utxo.value).sum::<u64>();
    Ok(balance)
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use anyhow::Result;
    use serde_json::json;
    use sqlx::{Sqlite, pool::PoolConnection};
    use zcash_protocol::consensus::Network;

    use crate::{
        balance::get_balance,
        context::Context,
        db::{create_schema, set_account_seed},
        lwd::{connect, scan_blocks}, pod::ElectionProps,
    };

    pub const TEST_SEED: &str = "path memory sun borrow real air lyrics way floor oblige beyond mouse wrap lyrics save doll slush rice absorb panel smile bid clog nephew";

    async fn setup() -> Result<PoolConnection<Sqlite>> {
        let ctx = Context::new("vote.db", "").await?;
        let mut conn = ctx.connect().await?;
        create_schema(&mut conn).await?;
        set_account_seed(&mut conn, TEST_SEED, 0).await?;
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
        e.store(&mut conn).await?;
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
        Ok(conn)
    }

    #[tokio::test]
    async fn test_question_balance() -> Result<()> {
        let mut conn = setup().await?;
        // Sleep to give some time for the scan to commit
        // the utxos to the db
        sleep(Duration::from_secs(1));
        let balance = get_balance(&mut conn, 3).await?;
        assert_eq!(balance, 1169078);
        Ok(())
    }
}
