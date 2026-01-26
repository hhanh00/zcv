use anyhow::Context;
use bip39::Mnemonic;
use ff::PrimeField;
use orchard::{
    Note,
    keys::{FullViewingKey, IncomingViewingKey, SpendingKey},
    vote::{Ballot, BallotData},
};
use pasta_curves::Fp;
use rocket::futures::StreamExt;
use sqlx::{Row, SqliteConnection, query, query_as};
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
        address TEXT NOT NULL,
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
        memo BLOB NOT NULL,
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
        id_ballot INTEGER PRIMARY KEY,
        height INTEGER NOT NULL,
        itx INTEGER NOT NULL,
        question INTEGER NOT NULL,
        data TEXT NOT NULL,
        witness TEXT NOT NULL,
        UNIQUE (height, itx))",
    )
    .execute(&mut *conn)
    .await?;
    // server / validator
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS results(
        question INTEGER NOT NULL,
        answer INTEGER NOT NULL,
        votes INTEGER NOT NULL,
        PRIMARY KEY (question, answer))",
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

pub async fn get_election(conn: &mut SqliteConnection, hash: &[u8]) -> ZCVResult<ElectionPropsPub> {
    let (election,): (String,) = query_as("SELECT data FROM elections WHERE hash = ?1")
        .bind(hash)
        .fetch_one(conn)
        .await
        .context("select election by hash")?;
    let domain: ElectionPropsPub = serde_json::from_str(&election)?;
    Ok(domain)
}

pub async fn get_domain(
    conn: &mut SqliteConnection,
    hash: &[u8],
    idx_question: usize,
) -> ZCVResult<(Fp, String)> {
    let (domain, address): (Vec<u8>, String) = query_as(
        "SELECT q.domain, q.address FROM questions q
        JOIN elections e ON q.election = e.id_election
    WHERE e.hash = ?1 AND q.idx = ?2",
    )
    .bind(hash)
    .bind(idx_question as u32)
    .fetch_one(conn)
    .await
    .context("select domain")?;
    let domain = Fp::from_repr(tiu!(domain)).unwrap();
    Ok((domain, address))
}

pub async fn get_apphash(conn: &mut SqliteConnection, hash: &[u8]) -> ZCVResult<Vec<u8>> {
    let (apphash,): (Vec<u8>,) = query_as("SELECT apphash FROM elections WHERE hash = ?1")
        .bind(hash)
        .fetch_one(conn)
        .await?;
    Ok(apphash)
}

pub async fn store_apphash(
    conn: &mut SqliteConnection,
    hash: &[u8],
    apphash: &[u8],
) -> ZCVResult<()> {
    query("UPDATE elections SET apphash = ?2 WHERE hash = ?1")
        .bind(hash)
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
    memo: &[u8],
    height: u32,
    position: u32,
    question: u32,
    scope: u32,
) -> ZCVResult<()> {
    let nf = note.nullifier(fvk);
    let dnf = note.nullifier_domain(fvk, election_domain);

    query(
        "INSERT INTO notes
    (question, height, scope, position, nf, dnf, rho, diversifier, rseed, value, memo)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(memo)
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
    itx: u32,
    ballot: Ballot,
) -> ZCVResult<()> {
    let Ballot { data, witnesses } = ballot;
    let domain = &data.domain;
    query(
        "INSERT INTO ballots(height, itx, question, data, witness)
    SELECT ?1, ?2, id_question, ?3, ?4 FROM questions
    WHERE domain = ?5 ON CONFLICT DO NOTHING",
    )
    .bind(height)
    .bind(itx)
    .bind(serde_json::to_string(&data)?)
    .bind(serde_json::to_string(&witnesses)?)
    .bind(domain)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn fetch_ballots(
    conn: &mut SqliteConnection,
    mut handler: impl AsyncFnMut(u32, BallotData) -> ZCVResult<()>,
) -> ZCVResult<()> {
    let mut s = query("SELECT question, data FROM ballots ORDER BY height, itx").fetch(conn);
    while let Some(r) = s.next().await {
        if let Ok(r) = r {
            let question: u32 = r.get(0);
            let data: String = r.get(1);
            let ballot_data: BallotData = serde_json::from_str(&data).unwrap();
            handler(question, ballot_data).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        db::{fetch_ballots, get_domain, get_election, set_account_seed, store_ballot},
        error::IntoAnyhow,
        tests::{TEST_ELECTION_HASH, get_connection, test_setup},
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
        let election = get_election(&mut conn, TEST_ELECTION_HASH).await?;
        let (domain, _address) = get_domain(&mut conn, TEST_ELECTION_HASH, 1).await?;
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
        store_ballot(&mut conn, election.end + 1, 0, dummy_ballot).await?;
        let (count_ballot,): (u32,) = query_as("SELECT COUNT(*) FROM ballots")
            .fetch_one(&mut *conn)
            .await?;
        assert_eq!(count_ballot, 1);

        let mut count_ballot2 = 0u32;
        fetch_ballots(&mut conn, async |_, ballot_data| {
            let h = ballot_data.sighash().anyhow()?;
            assert_eq!(
                hex::encode(&h),
                "aaf7c9385268beb9e936451d25b4327aa79d2c3239cd2f894bbb50eeccd44d42"
            );
            count_ballot2 += 1;
            Ok(())
        })
        .await?;
        assert_eq!(count_ballot, count_ballot2);
        Ok(())
    }
}
