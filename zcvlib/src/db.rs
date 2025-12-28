use anyhow::Context;
use bip39::Mnemonic;
use ff::PrimeField;
use orchard::{
    Note,
    keys::{FullViewingKey, IncomingViewingKey, SpendingKey},
    vote::Ballot,
};
use pasta_curves::Fp;
use sqlx::{SqliteConnection, query, query_as};
use zcash_protocol::consensus::{Network, NetworkConstants};
use zip32::{AccountId, Scope};

use crate::{
    ZCVResult,
    error::IntoAnyhow,
    pod::{ElectionPropsPub, QuestionPropPub},
    tiu,
};

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
        apphash BLOB NOT NULL,
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
        dnf BLOB NOT NULL,
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

    // server / validator
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ballots(
        height INTEGER PRIMARY KEY,
        question INTEGER NOT NULL,
        data TEXT NOT NULL,
        witness TEXT NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn set_account_seed(
    conn: &mut SqliteConnection,
    account: u32,
    mnemonic: &str,
    aindex: u32,
) -> ZCVResult<()> {
    Mnemonic::parse(mnemonic).anyhow()?;
    query(
        "INSERT INTO account(id_account, seed, aindex)
    VALUES (?1, ?2, ?3) ON CONFLICT DO UPDATE
    SET seed = excluded.seed, aindex = excluded.aindex",
    )
    .bind(account)
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
    let spk = derive_spending_key(network, &seed, aindex)?;
    let fvk = FullViewingKey::from(&spk);
    let ivks = (fvk.to_ivk(Scope::External), fvk.to_ivk(Scope::Internal));
    Ok((fvk, ivks.0, ivks.1))
}

pub async fn get_election(
    conn: &mut SqliteConnection,
    id_election: u32,
) -> ZCVResult<ElectionPropsPub> {
    let (election,): (String,) = query_as("SELECT data FROM elections WHERE id_election = ?1")
        .bind(id_election)
        .fetch_one(conn)
        .await
        .context("select election")?;
    let domain: ElectionPropsPub = serde_json::from_str(&election)?;
    Ok(domain)
}

pub async fn get_domain(
    conn: &mut SqliteConnection,
    id_election: u32,
    question: usize,
) -> ZCVResult<Fp> {
    let (domain,): (Vec<u8>,) = query_as(
        "SELECT domain FROM questions
    WHERE election = ?1 AND idx = ?2",
    )
    .bind(id_election)
    .bind(question as u32)
    .fetch_one(conn)
    .await
    .context("select domain")?;
    let domain = Fp::from_repr(tiu!(domain)).unwrap();
    Ok(domain)
}

pub async fn get_apphash(conn: &mut SqliteConnection, id_election: u32) -> ZCVResult<Vec<u8>> {
    let (apphash,): (Vec<u8>,) = query_as("SELECT apphash FROM elections WHERE id_election = ?1")
        .bind(id_election)
        .fetch_one(conn)
        .await?;
    Ok(apphash)
}

pub async fn store_apphash(conn: &mut SqliteConnection, id_election: u32, apphash: &[u8]) -> ZCVResult<()> {
    query("UPDATE elections SET apphash = ?2 WHERE id_election = ?1")
        .bind(id_election)
        .bind(apphash)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_question(conn: &mut SqliteConnection, domain: Fp) -> ZCVResult<QuestionPropPub> {
    let (data,): (String,) = query_as("SELECT data FROM questions WHERE domain = ?1")
        .bind(domain.to_repr().as_slice())
        .fetch_one(conn)
        .await?;
    let question: QuestionPropPub = serde_json::from_str(&data).unwrap();
    Ok(question)
}

pub fn derive_spending_key(network: &Network, seed: &str, aindex: u32) -> ZCVResult<SpendingKey> {
    let mnemonic = Mnemonic::parse(seed).anyhow()?;
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
    height: u32,
    position: u32,
    question: u32,
    scope: u32,
) -> ZCVResult<()> {
    let nf = note.nullifier(fvk);
    let dnf = note.nullifier_domain(fvk, election_domain);

    query(
        "INSERT INTO notes
    (question, height, scope, position, nf, dnf, rho, diversifier, rseed, value)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(question)
    .bind(height)
    .bind(scope)
    .bind(position)
    .bind(nf.to_bytes().as_slice())
    .bind(dnf.to_bytes().as_slice())
    .bind(note.rho().to_bytes().as_slice())
    .bind(note.recipient().diversifier().as_array().as_slice())
    .bind(note.rseed().as_bytes().as_slice())
    .bind(note.value().inner() as i64)
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn store_spend(
    conn: &mut SqliteConnection,
    id_question: u32,
    nf: &[u8],
    height: u32,
) -> ZCVResult<()> {
    query(
        "INSERT INTO spends
        (id_note, height, value)
        SELECT id_note, ?3, -value FROM notes WHERE question = ?1 AND nf = ?2",
    )
    .bind(id_question)
    .bind(nf)
    .bind(height)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn store_ballot(
    conn: &mut SqliteConnection,
    height: u32,
    ballot: Ballot,
) -> ZCVResult<()> {
    let Ballot { data, witnesses } = ballot;
    let domain = &data.domain;
    query(
        "INSERT INTO ballots(height, question, data, witness)
    SELECT ?1, id_question, ?2, ?3 FROM questions
    WHERE domain = ?4 ON CONFLICT DO NOTHING",
    )
    .bind(height)
    .bind(serde_json::to_string(&data)?)
    .bind(serde_json::to_string(&witnesses)?)
    .bind(domain)
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        db::{get_domain, get_election, set_account_seed, store_ballot},
        tests::{get_connection, test_setup},
    };
    use anyhow::Result;
    use ff::PrimeField;
    use orchard::vote::{Ballot, BallotAnchors, BallotData, BallotWitnesses};
    use sqlx::{query, query_as};

    #[tokio::test]
    async fn test_schema_creation() -> Result<()> {
        let mut conn = get_connection().await?;
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
        let mut conn = get_connection().await?;
        let r = set_account_seed(&mut conn, 0, "", 0).await;
        assert!(r.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_good_seed() -> Result<()> {
        let mut conn = get_connection().await?;
        let r = test_setup(&mut conn).await;
        assert!(r.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_store_ballot() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        query("DELETE FROM ballots").execute(&mut *conn).await?;
        let election = get_election(&mut conn, 1).await?;
        let domain = get_domain(&mut conn, 1, 1).await?;
        let dummy_ballot = Ballot {
            data: BallotData {
                version: 1,
                domain: domain.to_repr().to_vec(),
                actions: vec![],
                anchors: BallotAnchors {
                    nf: vec![0; 32],
                    cmx: vec![0; 32],
                },
            },
            witnesses: BallotWitnesses {
                proofs: vec![],
                sp_signatures: None,
                binding_signature: vec![],
            },
        };
        store_ballot(&mut conn, election.end + 1, dummy_ballot).await?;
        let (count_ballot,): (u32,) = query_as("SELECT COUNT(*) FROM ballots")
            .fetch_one(&mut *conn)
            .await?;
        assert_eq!(count_ballot, 1);
        Ok(())
    }
}
