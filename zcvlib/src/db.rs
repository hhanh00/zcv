use bip39::Mnemonic;
use sqlx::{SqliteConnection, query};

use crate::{ZCVResult, error::IntoAnyhow};

pub async fn create_schema(conn: &mut SqliteConnection) -> ZCVResult<()> {
    query(
        "CREATE TABLE IF NOT EXISTS account(
        id_account INTEGER PRIMARY KEY,
        seed TEXT NOT NULL,
        aindex INTEGER NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS elections(
        id_election INTEGER PRIMARY KEY,
        hash BLOB NOT NULL,
        data BLOB NOT NULL,
        UNIQUE (hash))",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS questions(
        id_question INTEGER PRIMARY KEY,
        election INTEGER NOT NULL,
        data TEXT NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS actions(
        id_action INTEGER PRIMARY KEY,
        height INTEGER NOT NULL,
        idx INTEGER NOT NULL,
        nf BLOB NOT NULL,
        cmx BLOB NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS notes(
        id_note INTEGER PRIMARY KEY,
        question INTEGER NOT NULL,
        height INTEGER NOT NULL,
        pool INTEGER NOT NULL,
        position INTEGER NOT NULL,
        nf BLOB NOT NULL,
        rho BLOB NOT NULL,
        diversifier BLOB NOT NULL,
        rseed BLOB NOT NULL,
        value INTEGER NOT NULL,
        UNIQUE (question, pool, position))",
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS spends(
        id_note INTEGER PRIMARY KEY,
        height INTEGER NOT NULL,
        value INTEGER NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn set_account_seed(conn: &mut SqliteConnection, mnemonic: &str, aindex: u32) -> ZCVResult<()> {
    Mnemonic::parse(mnemonic).anyhow()?;
    query("INSERT INTO account(id_account, seed, aindex)
    VALUES (0, ?1, ?2) ON CONFLICT DO UPDATE
    SET seed = excluded.seed, aindex = excluded.aindex")
    .bind(mnemonic)
    .bind(aindex)
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{context::Context, db::set_account_seed};
    use anyhow::Result;
    use sqlx::{Sqlite, pool::PoolConnection};

    async fn setup() -> Result<PoolConnection<Sqlite>> {
        let ctx = Context::new("vote.db", "").await?;
        let mut conn = ctx.connect().await?;
        super::create_schema(&mut conn).await?;
        Ok(conn)
    }

    #[tokio::test]
    async fn test_schema_creation() -> Result<()> {
        let mut conn = setup().await?;
        super::create_schema(&mut conn).await?;

        let (c,): (u32,) = sqlx::query_as(
            "SELECT 1 FROM sqlite_master WHERE type = 'table'
            AND name = 'elections'")
            .fetch_one(&mut *conn)
            .await?;

        assert_eq!(c, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_seed() -> Result<()> {
        let mut conn = setup().await?;
        let r = set_account_seed(&mut conn, "", 0).await;
        assert!(r.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_good_seed() -> Result<()> {
        let mut conn = setup().await?;
        let r = set_account_seed(&mut conn,
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", 0).await;
        assert!(r.is_ok());
        Ok(())
    }
}
