use bip39::Mnemonic;
use orchard::{
    Address, Note,
    keys::{FullViewingKey, IncomingViewingKey, SpendingKey},
};
use pasta_curves::Fp;
use sqlx::{SqliteConnection, query, query_as};
use zcash_protocol::consensus::{Network, NetworkConstants};
use zip32::{AccountId, Scope};

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
        name TEXT NOT NULL,
        start INTEGER NOT NULL,
        end INTEGER NOT NULL,
        need_sig BOOL NOT NULL,
        data TEXT NOT NULL,
        UNIQUE (hash))",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS questions(
        id_question INTEGER PRIMARY KEY,
        election INTEGER NOT NULL,
        idx INTEGER NOT NULL,
        domain BLOB NOT NULL,
        title TEXT NOT NULL,
        subtitle TEXT NOT NULL,
        data TEXT NOT NULL,
        UNIQUE (election, idx))",
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
        scope INTEGER NOT NULL,
        position INTEGER NOT NULL,
        nf BLOB NOT NULL,
        rho BLOB NOT NULL,
        diversifier BLOB NOT NULL,
        rseed BLOB NOT NULL,
        value INTEGER NOT NULL,
        UNIQUE (question, position))",
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

pub async fn set_account_seed(
    conn: &mut SqliteConnection,
    mnemonic: &str,
    aindex: u32,
) -> ZCVResult<()> {
    Mnemonic::parse(mnemonic).anyhow()?;
    query(
        "INSERT INTO account(id_account, seed, aindex)
    VALUES (0, ?1, ?2) ON CONFLICT DO UPDATE
    SET seed = excluded.seed, aindex = excluded.aindex",
    )
    .bind(mnemonic)
    .bind(aindex)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn get_ivks(
    network: &Network,
    conn: &mut SqliteConnection,
) -> ZCVResult<(FullViewingKey, IncomingViewingKey, IncomingViewingKey)> {
    let (seed, aindex): (String, u32) =
        query_as("SELECT seed, aindex FROM account WHERE id_account = 0")
            .fetch_one(conn)
            .await?;
    let spk = derive_spending_key(network, seed, aindex)?;
    let fvk = FullViewingKey::from(&spk);
    let ivks = (fvk.to_ivk(Scope::External), fvk.to_ivk(Scope::Internal));
    Ok((fvk, ivks.0, ivks.1))
}

fn derive_spending_key(network: &Network, seed: String, aindex: u32) -> ZCVResult<SpendingKey> {
    let mnemonic = Mnemonic::parse(&seed).anyhow()?;
    let seed = mnemonic.to_seed("");
    let spk = SpendingKey::from_zip32_seed(
        &seed,
        network.coin_type(),
        AccountId::const_from_u32(aindex),
    )
    .anyhow()?;
    Ok(spk)
}

#[allow(clippy::too_many_arguments)]
pub async fn store_received_note(
    conn: &mut SqliteConnection,
    election_domain: Fp,
    fvk: &FullViewingKey,
    note: &Note,
    address: &Address,
    pre_snapshot: bool,
    height: u32,
    position: u32,
    question: u32,
    scope: u32,
) -> ZCVResult<()> {
    let nf = if pre_snapshot {
        note.nullifier(fvk)
    } else {
        note.nullifier_domain(fvk, election_domain)
    };

    query(
        "INSERT INTO notes
    (question, height, scope, position, nf, rho, diversifier, rseed, value)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(question)
    .bind(height)
    .bind(scope)
    .bind(position)
    .bind(nf.to_bytes().as_slice())
    .bind(note.rho().to_bytes().as_slice())
    .bind(address.diversifier().as_array().as_slice())
    .bind(note.rseed().as_bytes().as_slice())
    .bind(note.value().inner() as i64)
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
            AND name = 'elections'",
        )
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
