use std::pin::Pin;

use anyhow::Context;
use bech32::{Bech32m, Hrp};
use bip39::Mnemonic;
use ff::PrimeField;
use futures::StreamExt;
use orchard::{
    Note,
    keys::{FullViewingKey, IncomingViewingKey, SpendingKey},
    vote::{Ballot, BallotData, BallotWitnesses},
};
use pasta_curves::Fp;
use sqlx::{Row, SqliteConnection, query, query_as, sqlite::SqliteRow};
use zcash_protocol::consensus::{Network, NetworkConstants};
use zip32::{AccountId, Scope};

use crate::{
    ZCVResult,
    error::IntoAnyhow,
    pod::{ElectionPropsPub, QuestionPropPub, ZCV_HRP},
    tiu,
};

pub async fn create_schema(conn: &mut SqliteConnection) -> ZCVResult<()> {
    query(
        "CREATE TABLE IF NOT EXISTS state(
        id INTEGER PRIMARY KEY,
        hash BLOB NOT NULL,
        started BOOL NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "INSERT INTO state(id, hash, started)
    VALUES (0, '', FALSE) ON CONFLICT DO NOTHING",
    )
    .execute(&mut *conn)
    .await?;
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
        height INTEGER NOT NULL,
        position INTEGER NOT NULL,
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
        account INTEGER NOT NULL,
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
        data BLOB NOT NULL,
        witnesses BLOB NOT NULL,
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

pub async fn get_account_address(network: &Network, conn: &mut SqliteConnection, id_account: u32) -> ZCVResult<String> {
    let (seed, aindex): (String, u32) = query_as("SELECT seed, aindex FROM account WHERE id_account = ?1")
    .bind(id_account)
    .fetch_one(conn)
    .await?;
    let sk = derive_spending_key(network, &seed, aindex)?;
    let fvk = FullViewingKey::from(&sk);
    let address = fvk.address_at(0u64, Scope::External);
    let hrp = Hrp::parse(ZCV_HRP).anyhow()?;
    let address = bech32::encode::<Bech32m>(hrp, &address.to_raw_address_bytes()).anyhow()?;
    Ok(address)
}

pub async fn store_election(
    conn: &mut SqliteConnection,
    election: &ElectionPropsPub,
) -> ZCVResult<u32> {
    let hash = election.hash()?;
    let json = serde_json::to_string(election).anyhow()?;
    let (id_election,): (u32,) = query_as(
        "INSERT INTO elections
            (hash, apphash, start, end, height, position, need_sig, name, data)
            VALUES (?, '', ?, ?, ?, 0, ?, ?, ?) ON CONFLICT DO UPDATE SET
            start = excluded.start,
            end = excluded.end,
            height = excluded.height,
            need_sig = excluded.need_sig,
            name = excluded.name,
            data = excluded.data
            RETURNING id_election",
    )
    .bind(hash.as_slice())
    .bind(election.start)
    .bind(election.end)
    .bind(election.start - 1)
    .bind(election.need_sig)
    .bind(&election.name)
    .bind(&json)
    .fetch_one(&mut *conn)
    .await?;
    for (i, q) in election.questions.iter().enumerate() {
        let q_js = serde_json::to_string(q).anyhow()?;
        let domain = q.domain(election)?;
        query(
            "INSERT INTO questions
                (election, idx, domain, address, title, subtitle, data)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT DO UPDATE SET
                domain = excluded.domain,
                address = excluded.address,
                title = excluded.title,
                subtitle = excluded.subtitle,
                data = excluded.data",
        )
        .bind(id_election)
        .bind(i as u32)
        .bind(domain.to_repr().as_slice())
        .bind(&q.address)
        .bind(&q.title)
        .bind(&q.subtitle)
        .bind(q_js)
        .execute(&mut *conn)
        .await?;
    }
    Ok(id_election)
}

pub async fn get_ivks(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
) -> ZCVResult<(FullViewingKey, IncomingViewingKey, IncomingViewingKey)> {
    let (seed, aindex): (String, u32) =
        query_as("SELECT seed, aindex FROM account WHERE id_account = ?1")
            .bind(id_account)
            .fetch_one(conn)
            .await?;
    let spk = derive_spending_key(network, &seed, aindex)?;
    let fvk = FullViewingKey::from(&spk);
    let ivks = (fvk.to_ivk(Scope::External), fvk.to_ivk(Scope::Internal));
    Ok((fvk, ivks.0, ivks.1))
}

pub async fn set_election(conn: &mut SqliteConnection, hash: &[u8]) -> ZCVResult<()> {
    query("UPDATE state SET hash = ?1 WHERE id = 0")
        .bind(hash)
        .execute(conn)
        .await?;
    Ok(())
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

pub async fn get_apphash(conn: &mut SqliteConnection, hash: &[u8]) -> ZCVResult<Option<Vec<u8>>> {
    let apphash = query_as::<_, (Vec<u8>,)>("SELECT apphash FROM elections WHERE hash = ?1")
        .bind(hash)
        .fetch_optional(conn)
        .await?
        .map(|v| v.0);
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

pub async fn store_election_height_position(
    db_tx: &mut SqliteConnection,
    hash: &[u8],
    height: u32,
    position: u32,
) -> ZCVResult<()> {
    query("UPDATE elections SET height = ?1, position = ?2 WHERE hash = ?3")
        .bind(height)
        .bind(position)
        .bind(hash)
        .execute(db_tx)
        .await?;
    Ok(())
}

pub async fn store_election_height_inc_position(
    db_tx: &mut SqliteConnection,
    hash: &[u8],
    height: u32,
) -> ZCVResult<()> {
    query("UPDATE elections SET height = ?1, position = position + 1 WHERE hash = ?2")
        .bind(height)
        .bind(hash)
        .execute(db_tx)
        .await?;
    Ok(())
}

pub async fn get_election_height(conn: &mut SqliteConnection, hash: &[u8]) -> ZCVResult<u32> {
    let (height,): (u32,) = query_as("SELECT height FROM elections WHERE hash = ?1")
        .bind(hash)
        .fetch_one(conn)
        .await?;
    Ok(height)
}

pub async fn get_election_position(conn: &mut SqliteConnection, hash: &[u8]) -> ZCVResult<u32> {
    let (position, ): (u32, ) = query_as("SELECT position FROM elections WHERE hash = ?1")
    .bind(hash)
    .fetch_one(conn)
    .await?;
    Ok(position)
}

pub async fn get_question(conn: &mut SqliteConnection, domain: Fp) -> ZCVResult<QuestionPropPub> {
    let (data,): (String,) = query_as("SELECT data FROM questions WHERE domain = ?1")
        .bind(domain.to_repr().as_slice())
        .fetch_one(conn)
        .await?;
    let question: QuestionPropPub = serde_json::from_str(&data).unwrap();
    Ok(question)
}

pub async fn list_unspent_nullifiers(
    conn: &mut SqliteConnection,
    id_account: u32,
) -> ZCVResult<Vec<Vec<u8>>> {
    let dnfs = query(
        "SELECT n.dnf FROM notes n LEFT JOIN spends s ON n.id_note = s.id_note
        WHERE s.id_note IS NULL
        AND n.account = ?1",
    )
    .bind(id_account)
    .map(|r: SqliteRow| {
        let dnf: Vec<u8> = r.get(0);
        dnf
    })
    .fetch_all(conn)
    .await?;
    Ok(dnfs)
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

pub async fn delete_range(conn: &mut SqliteConnection, start: u32, end: u32) -> ZCVResult<()> {
    query("DELETE FROM notes WHERE height >= ?1 AND height <= ?2")
    .bind(start)
    .bind(end)
    .execute(conn)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn store_received_note(
    conn: &mut SqliteConnection,
    election_domain: Fp,
    id_account: u32,
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
    (account, question, height, scope, position, nf, dnf, rho, diversifier, rseed, value, memo)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id_account)
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

pub async fn store_ballot_spend(
    conn: &mut SqliteConnection,
    id_question: u32,
    dnf: &[u8],
    height: u32,
) -> ZCVResult<()> {
    query(
        "INSERT INTO spends
        (id_note, height, value)
        SELECT id_note, ?3, -value FROM notes WHERE question = ?1 AND dnf = ?2",
    )
    .bind(id_question)
    .bind(dnf)
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
) -> ZCVResult<u32> {
    let Ballot { data, witnesses } = ballot;
    let domain = &data.domain;
    let mut data_bytes = vec![];
    data.write(&mut data_bytes).anyhow()?;
    let mut witnesses_bytes = vec![];
    witnesses.write(&mut witnesses_bytes).anyhow()?;

    let r = query(
        "INSERT INTO ballots(height, itx, question, data, witnesses)
    SELECT ?1, ?2, id_question, ?3, ?4 FROM questions
    WHERE domain = ?5 ON CONFLICT DO NOTHING",
    )
    .bind(height)
    .bind(itx)
    .bind(&data_bytes)
    .bind(&witnesses_bytes)
    .bind(domain.as_slice())
    .execute(conn)
    .await?;
    Ok(r.rows_affected() as u32)
}

pub async fn fetch_ballots(
    conn: &mut SqliteConnection,
    mut handler: impl AsyncFnMut(u32, BallotData) -> ZCVResult<()>,
) -> ZCVResult<()> {
    let mut s = query("SELECT question, data FROM ballots ORDER BY height, itx").fetch(conn);
    while let Some(r) = s.next().await {
        if let Ok(r) = r {
            let question: u32 = r.get(0);
            let data: Vec<u8> = r.get(1);
            let ballot_data = BallotData::read(&*data).unwrap();
            handler(question, ballot_data).await?;
        }
    }
    Ok(())
}

pub async fn get_ballot_range(
    mut conn: SqliteConnection,
    start: u32,
    end: u32,
    handler: impl Fn(crate::vote_rpc::Ballot) -> Pin<Box<dyn Future<Output = ZCVResult<()>> + Send>> + 'static + Send + Sync
) -> ZCVResult<()> {
    tokio::spawn(async move {
        let mut s = query(
            "SELECT height, itx, data, witnesses FROM ballots
    WHERE height >= ?1 AND height <= ?2 ORDER BY height, itx",
        )
        .bind(start)
        .bind(end)
        .fetch(&mut conn);
        while let Some(r) = s.next().await {
            if let Ok(r) = r {
                let height: u32 = r.get(0);
                let itx: u32 = r.get(1);
                let data: Vec<u8> = r.get(2);
                let witnesses: Vec<u8> = r.get(3);
                let data = BallotData::read(&*data)?;
                let witnesses = BallotWitnesses::read(&*witnesses)?;
                let b = Ballot {
                    data,
                    witnesses,
                };
                let mut ballot = vec![];
                b.write(&mut ballot)?;
                let ballot = crate::vote_rpc::Ballot {
                    height,
                    itx,
                    ballot,
                };
                handler(ballot).await?;
            }
        }
        Ok::<_, anyhow::Error>(())
    });
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
                domain: domain.to_repr(),
                actions: vec![],
                anchors: BallotAnchors {
                    nf: [0; 32],
                    cmx: [0; 32],
                },
            },
            witnesses: BallotWitnesses {
                proofs: vec![],
                sp_signatures: None,
                binding_signature: [0u8; 64],
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
                "942bc20fdda82c173dd2cd38033a62c96ee7424e47dc1e214186cc5d179caa67"
            );
            count_ballot2 += 1;
            Ok(())
        })
        .await?;
        assert_eq!(count_ballot, count_ballot2);
        Ok(())
    }
}
