use anyhow::Context;
use pasta_curves::Fp;
use ff::PrimeField;
use sqlx::{Row, SqliteConnection, query, sqlite::SqliteRow};

use crate::{ZCVResult, pod::UTXO};

pub async fn list_unspent_notes(
    conn: &mut SqliteConnection,
    domain: Fp,
    id_account: u32,
) -> ZCVResult<Vec<UTXO>> {
    let utxos = query(
        "SELECT n.height, scope, position, nf, dnf, rho, diversifier, rseed, n.value
        FROM v_notes n LEFT JOIN v_spends s ON n.id_note = s.id_note
        JOIN v_questions q ON q.id_question = n.question
        WHERE s.id_note IS NULL AND q.domain = ?1
        AND n.account = ?2",
    )
    .bind(domain.to_repr().as_slice())
    .bind(id_account)
    .map(|r: SqliteRow| {
        let height: u32 = r.get(0);
        let scope: u32 = r.get(1);
        let position: u32 = r.get(2);
        let nf: Vec<u8> = r.get(3);
        let dnf: Vec<u8> = r.get(4);
        let rho: Vec<u8> = r.get(5);
        let diversifier: Vec<u8> = r.get(6);
        let rseed: Vec<u8> = r.get(7);
        let value: u64 = r.get(8);
        UTXO {
            height,
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
    .await
    .context("list_unspent_notes")?;
    Ok(utxos)
}

pub async fn get_balance(conn: &mut SqliteConnection, domain: Fp, id_account: u32) -> ZCVResult<u64> {
    let utxos = list_unspent_notes(conn, domain, id_account).await?;
    let balance = utxos.iter().map(|utxo| utxo.value).sum::<u64>();
    Ok(balance)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::{
        balance::get_balance, db::get_domain, tests::{TEST_ELECTION_HASH, get_connection, run_scan, test_setup}
    };

    #[tokio::test]
    #[serial_test::serial]
    async fn test_question_balance() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        let (domain, _) = get_domain(&mut conn, TEST_ELECTION_HASH, 2).await?;
        let balance = get_balance(&mut conn, domain, 0).await?;
        assert_eq!(balance, 1169078);
        Ok(())
    }
}
