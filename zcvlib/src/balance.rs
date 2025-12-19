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

    use crate::{
        balance::get_balance,
        tests::{get_connection, run_scan, test_setup},
    };

    #[tokio::test]
    #[serial_test::serial]
    async fn test_question_balance() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        // Sleep to give some time for the scan to commit
        // the utxos to the db
        sleep(Duration::from_secs(1));
        let balance = get_balance(&mut conn, 3).await?;
        assert_eq!(balance, 1169078);
        Ok(())
    }
}
