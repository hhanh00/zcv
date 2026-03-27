use std::pin::Pin;

use anyhow::{Context, anyhow};
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
use sqlx::{Acquire, Row, SqliteConnection, query, query_as, sqlite::SqliteRow};
use zcash_protocol::consensus::{Network, NetworkConstants};
use zip32::{AccountId, Scope};

use crate::{
    ZCVResult,
    error::IntoAnyhow,
    pod::{ElectionPropsPub, ZCV_HRP},
    tiu,
};

async fn column_exists(
    conn: &mut SqliteConnection,
    table: &str,
    column: &str,
) -> ZCVResult<Option<bool>> {
    let rows = query(&format!("PRAGMA table_info({})", table))
        .fetch_all(conn)
        .await?;
    let exists = rows.iter().any(|row| {
        let name: &str = row.try_get("name").unwrap_or_default();
        name == column
    });
    Ok(Some(exists))
}

pub async fn drop_schema(conn: &mut SqliteConnection) -> ZCVResult<()> {
    for table in &[
        "v_state",
        "v_elections",
        "v_notes",
        "v_spends",
        "v_ballots",
        "v_actions",
        "vc_nfs",
        "vc_cmxs",
        "v_results",
        "v_final_results",
    ] {
        query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn create_schema(conn: &mut SqliteConnection) -> ZCVResult<()> {
    let mut version = if let Some(has_version) = column_exists(conn, "v_state", "version").await?
        && has_version
    {
        let (version,): (u32,) = query_as("SELECT version FROM v_state WHERE id = 0")
            .fetch_one(&mut *conn)
            .await?;
        version
    } else {
        0
    };

    // Work around schema change prior to version tag
    if column_exists(conn, "v_state", "locked").await? == Some(true) {
        version = 1;
    }

    if version != 1 {
        drop_schema(&mut *conn).await?;
    }

    query(
        "CREATE TABLE IF NOT EXISTS v_state(
        id INTEGER PRIMARY KEY,
        version INTEGER,
        account INTEGER,
        url TEXT,
        apphash BLOB,
        height INTEGER,
        locked BOOL NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;

    let _ = query("ALTER TABLE v_state ADD COLUMN version INTEGER")
        .execute(&mut *conn)
        .await;

    let _ = query("ALTER TABLE v_elections ADD COLUMN nf_root BLOB")
        .execute(&mut *conn)
        .await;

    let _ = query("ALTER TABLE v_elections ADD COLUMN cmx_tree BLOB")
        .execute(&mut *conn)
        .await;

    query(
        "INSERT INTO v_state(id, version, locked)
    VALUES (0, 1, FALSE) ON CONFLICT DO NOTHING",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS accounts(
        id_account INTEGER PRIMARY KEY,
        seed TEXT NOT NULL,
        aindex INTEGER NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS v_elections(
        id_election INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        start INTEGER NOT NULL,
        end INTEGER NOT NULL,
        height INTEGER NOT NULL,
        need_sig BOOL NOT NULL,
        domain BLOB NOT NULL,
        address TEXT NOT NULL,
        data TEXT NOT NULL,
        position INTEGER NOT NULL,
        UNIQUE (domain))",
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS v_notes(
        id_note INTEGER PRIMARY KEY,
        account INTEGER NOT NULL,
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
        UNIQUE (position))",
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS v_spends(
        id_note INTEGER PRIMARY KEY,
        height INTEGER NOT NULL,
        value INTEGER NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;

    // server / validator
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS v_ballots(
        id_ballot INTEGER PRIMARY KEY,
        height INTEGER NOT NULL,
        itx INTEGER NOT NULL,
        data BLOB NOT NULL,
        witnesses BLOB NOT NULL,
        UNIQUE (height, itx))",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS vs_cmxs(
        cmx BLOB PRIMARY KEY NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS v_actions(
        id_action INTEGER PRIMARY KEY,
        height INTEGER NOT NULL,
        ballot INTEGER NOT NULL,
        idx INTEGER NOT NULL,
        dnf BLOB NOT NULL,
        cmx BLOB NOT NULL,
        UNIQUE (dnf))",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS vc_nfs(
        id_nf INTEGER PRIMARY KEY,
        nf BLOB NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    query(
        "CREATE TABLE IF NOT EXISTS vc_cmxs(
        id_cmx INTEGER PRIMARY KEY,
        cmx BLOB NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    // server / validator
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS v_results(
        id_result INTEGER PRIMARY KEY,
        answer BLOB NOT NULL,
        votes INTEGER NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS v_final_results(
        idx_question INTEGER NOT NULL,
        idx_answer INTEGER NOT NULL,
        votes INTEGER NOT NULL,
        PRIMARY KEY (idx_question, idx_answer))",
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
        "INSERT INTO accounts(id_account, seed, aindex)
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

pub async fn get_account_sk(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
) -> ZCVResult<SpendingKey> {
    let (seed, aindex): (String, u32) =
        query_as("SELECT seed, aindex FROM accounts WHERE id_account = ?1")
            .bind(id_account)
            .fetch_one(conn)
            .await
            .context("get_account_address")?;
    let sk = derive_spending_key(network, &seed, aindex)?;
    Ok(sk)
}

pub async fn get_account_address(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
) -> ZCVResult<String> {
    let sk = get_account_sk(network, conn, id_account).await?;
    let fvk = FullViewingKey::from(&sk);
    let address = fvk.address_at(0u64, Scope::External);
    let hrp = Hrp::parse(ZCV_HRP).anyhow()?;
    let address = bech32::encode::<Bech32m>(hrp, &address.to_raw_address_bytes()).anyhow()?;
    Ok(address)
}

pub async fn store_election(
    conn: &mut SqliteConnection,
    election: &ElectionPropsPub,
) -> ZCVResult<()> {
    let json = serde_json::to_string(election).anyhow()?;
    query(
        "INSERT INTO v_elections
            (id_election, domain, start, end, height, position, need_sig, name, address, data)
            VALUES (0, ?, ?, ?, ?, 0, ?, ?, ?, ?) ON CONFLICT DO UPDATE SET
            domain = excluded.domain,
            start = excluded.start,
            end = excluded.end,
            need_sig = excluded.need_sig,
            name = excluded.name,
            data = excluded.data,
            address = excluded.address",
        // Do not reset the height
    )
    .bind(election.domain.as_slice())
    .bind(election.start)
    .bind(election.end)
    .bind(election.start - 1)
    .bind(election.need_sig)
    .bind(&election.name)
    .bind(&election.address)
    .bind(&json)
    .execute(&mut *conn)
    .await
    .context("store_election")?;
    Ok(())
}

pub async fn client_delete_election_data(
    conn: &mut SqliteConnection,
    new_account: Option<u32>,
) -> ZCVResult<()> {
    let mut db_tx = conn.begin().await?;
    query("UPDATE v_state SET account = ?1 WHERE id = 0")
        .bind(new_account)
        .execute(&mut *db_tx)
        .await?;
    query("DELETE FROM v_notes").execute(&mut *db_tx).await?;
    query("DELETE FROM v_spends").execute(&mut *db_tx).await?;
    query("DELETE FROM vc_nfs").execute(&mut *db_tx).await?;
    query("DELETE FROM vc_cmxs").execute(&mut *db_tx).await?;
    query("UPDATE v_elections SET height = start - 1 WHERE id_election = 0")
        .execute(&mut *db_tx)
        .await?;

    db_tx.commit().await?;
    Ok(())
}

pub async fn client_delete_election(conn: &mut SqliteConnection) -> ZCVResult<()> {
    let mut db_tx = conn.begin().await?;
    query(
        "UPDATE v_state SET url = NULL,
    account = NULL, locked = 0 WHERE id = 0",
    )
    .execute(&mut *db_tx)
    .await?;
    query("DELETE FROM v_elections")
        .execute(&mut *db_tx)
        .await?;
    query("DELETE FROM v_notes").execute(&mut *db_tx).await?;
    query("DELETE FROM v_spends").execute(&mut *db_tx).await?;
    query("DELETE FROM vc_nfs").execute(&mut *db_tx).await?;
    query("DELETE FROM vc_cmxs").execute(&mut *db_tx).await?;

    db_tx.commit().await?;
    Ok(())
}

pub async fn get_ivks(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
) -> ZCVResult<(FullViewingKey, IncomingViewingKey, IncomingViewingKey)> {
    let (seed, aindex): (String, u32) =
        query_as("SELECT seed, aindex FROM accounts WHERE id_account = ?1")
            .bind(id_account)
            .fetch_one(conn)
            .await
            .context("get_ivks")?;
    let spk = derive_spending_key(network, &seed, aindex)?;
    let fvk = FullViewingKey::from(&spk);
    let ivks = (fvk.to_ivk(Scope::External), fvk.to_ivk(Scope::Internal));
    Ok((fvk, ivks.0, ivks.1))
}

pub async fn get_election(conn: &mut SqliteConnection) -> ZCVResult<ElectionPropsPub> {
    get_election_opt(conn)
        .await?
        .ok_or(anyhow!("No Election Set").into())
}

pub async fn get_election_opt(conn: &mut SqliteConnection) -> ZCVResult<Option<ElectionPropsPub>> {
    let election = query("SELECT data FROM v_elections WHERE id_election = 0")
        .map(|r: SqliteRow| r.get::<String, _>(0))
        .fetch_optional(conn)
        .await
        .context("get_election")?;
    let domain = election
        .map(|e| serde_json::from_str::<ElectionPropsPub>(&e))
        .transpose()?;
    Ok(domain)
}

pub async fn get_domain(conn: &mut SqliteConnection) -> ZCVResult<(Fp, String)> {
    let (domain, address): (Vec<u8>, String) = query_as(
        "SELECT domain, address FROM v_elections
        WHERE id_election = 0",
    )
    .fetch_one(conn)
    .await
    .context("select domain")?;
    let domain = Fp::from_repr(tiu!(domain)).unwrap();
    Ok((domain, address))
}

pub async fn get_apphash(conn: &mut SqliteConnection) -> ZCVResult<(Option<u32>, Option<Vec<u8>>)> {
    let (height, apphash) = query("SELECT height, apphash FROM v_state WHERE id = 0")
        .map(|r: SqliteRow| {
            let height: Option<u32> = r.get(0);
            let apphash: Option<Vec<u8>> = r.get(1);
            (height, apphash)
        })
        .fetch_one(conn)
        .await?;
    Ok((height, apphash))
}

pub async fn store_apphash(
    conn: &mut SqliteConnection,
    height: u32,
    apphash: &[u8],
) -> ZCVResult<()> {
    query("UPDATE v_state SET height = ?1, apphash = ?2 WHERE id = 0")
        .bind(height)
        .bind(apphash)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_roots(conn: &mut SqliteConnection) -> ZCVResult<Option<(Vec<u8>, Vec<u8>)>> {
    let (nf_root, cmx_tree) = query("SELECT nf_root, cmx_tree FROM v_elections WHERE id_election = 0")
        .map(|r: SqliteRow| {
            let nf_root: Option<Vec<u8>> = r.get(0);
            let cmx_tree: Option<Vec<u8>> = r.get(1);
            (nf_root, cmx_tree)
        })
        .fetch_one(&mut *conn)
        .await?;
    if let Some(nf_root) = nf_root {
        return Ok(Some((nf_root, cmx_tree.unwrap())));
    }
    Ok(None)
}

pub async fn store_roots(
    conn: &mut SqliteConnection,
    nf_root: &[u8],
    cmx_tree: &[u8],
) -> ZCVResult<()> {
    query("UPDATE v_elections SET nf_root = ?1, cmx_tree = ?2 WHERE id_election = 0")
        .bind(nf_root)
        .bind(cmx_tree)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn store_cmx_root(conn: &mut SqliteConnection, cmx: &[u8]) -> ZCVResult<()> {
    query("INSERT INTO vs_cmxs(cmx) VALUES (?1) ON CONFLICT DO NOTHING")
        .bind(cmx)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn check_cmx_root(conn: &mut SqliteConnection, cmx_root: &[u8]) -> ZCVResult<()> {
    let exist: Option<(bool,)> = query_as("SELECT 1 FROM vs_cmxs WHERE cmx = ?1")
        .bind(cmx_root)
        .fetch_optional(conn)
        .await?;
    tracing::info!("check_cmx_root {exist:?}");
    if exist.is_none() {
        return Err(crate::ZCVError::Any(anyhow!("Unknown cmx_root")));
    }
    Ok(())
}

pub async fn store_election_height_position(
    db_tx: &mut SqliteConnection,
    height: u32,
    position: u32,
) -> ZCVResult<()> {
    query("UPDATE v_elections SET height = ?1, position = ?2 WHERE id_election = 0")
        .bind(height)
        .bind(position)
        .execute(db_tx)
        .await?;
    Ok(())
}

pub async fn store_election_height_inc_position(
    db_tx: &mut SqliteConnection,
    height: u32,
) -> ZCVResult<()> {
    query("UPDATE v_elections SET height = ?1, position = position + 1 WHERE id_election = 0")
        .bind(height)
        .execute(db_tx)
        .await?;
    Ok(())
}

pub async fn get_election_height(conn: &mut SqliteConnection) -> ZCVResult<u32> {
    let (height,): (u32,) = query_as("SELECT height FROM v_elections WHERE id_election = 0")
        .fetch_one(conn)
        .await
        .context("get election height")?;
    Ok(height)
}

pub async fn get_election_position(conn: &mut SqliteConnection) -> ZCVResult<u32> {
    let (position,): (u32,) = query_as("SELECT position FROM v_elections WHERE id_election = 0")
        .fetch_one(conn)
        .await
        .context("get election position")?;
    Ok(position)
}

pub async fn list_unspent_nullifiers(
    conn: &mut SqliteConnection,
    id_account: u32,
) -> ZCVResult<Vec<Vec<u8>>> {
    let dnfs = query(
        "SELECT n.dnf FROM v_notes n LEFT JOIN v_spends s ON n.id_note = s.id_note
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
    query("DELETE FROM v_notes WHERE height >= ?1 AND height <= ?2")
        .bind(start)
        .bind(end)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn store_nf_cmx(
    conn: &mut SqliteConnection,
    nullifier: &[u8],
    cmx: &[u8],
) -> ZCVResult<()> {
    query(
        "INSERT INTO vc_nfs(nf) VALUES (?1)
        ON CONFLICT DO NOTHING",
    )
    .bind(nullifier)
    .execute(&mut *conn)
    .await?;
    query(
        "INSERT INTO vc_cmxs(cmx) VALUES (?1)
        ON CONFLICT DO NOTHING",
    )
    .bind(cmx)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn store_cmx(conn: &mut SqliteConnection, cmx: &[u8]) -> ZCVResult<()> {
    query(
        "INSERT INTO vc_cmxs(cmx) VALUES (?1)
        ON CONFLICT DO NOTHING",
    )
    .bind(cmx)
    .execute(&mut *conn)
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
    scope: u32,
) -> ZCVResult<()> {
    let nf = note.nullifier(fvk);
    let dnf = note.nullifier_domain(fvk, election_domain);

    query(
        "INSERT INTO v_notes
    (account, height, scope, position, nf, dnf, rho, diversifier, rseed, value, memo)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id_account)
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

pub async fn store_spend(conn: &mut SqliteConnection, nf: &[u8], height: u32) -> ZCVResult<()> {
    query(
        "INSERT INTO v_spends
        (id_note, height, value)
        SELECT id_note, ?2, -value FROM v_notes WHERE nf = ?1",
    )
    .bind(nf)
    .bind(height)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn store_ballot_spend(
    conn: &mut SqliteConnection,
    id_account: u32,
    dnf: &[u8],
    height: u32,
) -> ZCVResult<()> {
    query(
        "INSERT INTO v_spends
        (id_note, height, value)
        SELECT id_note, ?3, -value FROM v_notes WHERE account = ?1 AND dnf = ?2",
    )
    .bind(id_account)
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
) -> ZCVResult<Option<u32>> {
    let Ballot { data, witnesses } = ballot;
    let mut data_bytes = vec![];
    data.write(&mut data_bytes).anyhow()?;
    let mut witnesses_bytes = vec![];
    witnesses.write(&mut witnesses_bytes).anyhow()?;

    let mut db_tx = conn.begin().await?;
    let id_ballot = query(
        "INSERT INTO v_ballots(height, itx, data, witnesses)
    VALUES (?, ?, ?, ?)
    ON CONFLICT DO NOTHING
    RETURNING id_ballot",
    )
    .bind(height)
    .bind(itx)
    .bind(&data_bytes)
    .bind(&witnesses_bytes)
    .map(|r: SqliteRow| r.get::<u32, _>(0))
    .fetch_optional(&mut *db_tx)
    .await
    .context("store ballot")?;
    if let Some(id_ballot) = id_ballot {
        for (idx, a) in data.actions.iter().enumerate() {
            query(
                "INSERT INTO v_actions(ballot, idx, height, dnf, cmx)
        VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(id_ballot)
            .bind(idx as u32)
            .bind(height)
            .bind(a.nf.as_slice())
            .bind(a.cmx.as_slice())
            .execute(&mut *db_tx)
            .await?;
        }
    }
    db_tx.commit().await?;
    Ok(id_ballot)
}

pub async fn get_ballot_range(
    mut conn: SqliteConnection,
    start: u32,
    end: u32,
    handler: impl Fn(crate::vote_rpc::Ballot) -> Pin<Box<dyn Future<Output = ZCVResult<()>> + Send>>
    + 'static
    + Send
    + Sync,
) -> ZCVResult<()> {
    tokio::spawn(async move {
        let mut s = query(
            "SELECT height, itx, data, witnesses FROM v_ballots
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
                let b = Ballot { data, witnesses };
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

pub async fn store_result(conn: &mut SqliteConnection, memo: &[u8], value: u64) -> ZCVResult<()> {
    query(
        "INSERT INTO v_results(answer, votes)
    VALUES (?1, ?2)",
    )
    .bind(memo)
    .bind(value as i64)
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
            AND name = 'v_elections'",
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
        query("DELETE FROM v_ballots").execute(&mut *conn).await?;
        let election = get_election(&mut conn).await?;
        let (domain, _address) = get_domain(&mut conn).await?;
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
        let (count_ballot,): (u32,) = query_as("SELECT COUNT(*) FROM v_ballots")
            .fetch_one(&mut *conn)
            .await?;
        assert_eq!(count_ballot, 1);
        Ok(())
    }
}
