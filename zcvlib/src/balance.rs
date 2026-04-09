use anyhow::Context;
use bincode::config::legacy;
use ff::PrimeField;
use pasta_curves::Fp;
use pir_client::PirClient;
use sqlx::{Row, SqliteConnection, query, sqlite::SqliteRow};
use tonic::Request;
use zcash_protocol::consensus::Network;
use zcash_trees::warp::{Witness, hasher::OrchardHasher, legacy::CommitmentTreeFrontier};

use crate::{
    ZCVResult,
    db::{get_ivks, note_from_parts, store_received_note},
    error::IntoAnyhow,
    lwd::Client,
    pod::{ImtProofDataBin, UTXO},
    rpc::BlockId,
};

pub async fn check_witnesses(conn: &mut SqliteConnection, account: u32, height: u32) -> ZCVResult<bool> {
    let n = query(
        "SELECT n.id_note, MAX(w.height) AS max_witness_height
        FROM notes n
        JOIN witnesses w ON w.note = n.id_note AND w.account = n.account
        LEFT JOIN spends s ON s.id_note = n.id_note AND s.height < ?1
        WHERE n.account = ?2
        AND n.height < ?1
        AND n.pool = 2
        AND s.id_note IS NULL
        GROUP BY n.id_note",
    )
    .bind(height)
    .bind(account)
    .map(|r: SqliteRow| {
        let id_note: u32 = r.get(0);
        let height: u32 = r.get(1);
        (id_note, height)
    })
    .fetch_all(conn)
    .await?;
    Ok(n.iter().all(|(_, witness_height)| *witness_height >= height))
}

pub async fn import_account(
    network: &Network,
    conn: &mut SqliteConnection,
    client: &mut Client,
    pir_client: &PirClient,
    account: u32,
    domain: Fp,
    height: u32,
) -> ZCVResult<()> {
    query("DELETE FROM v_notes").execute(&mut *conn).await?;
    query("DELETE FROM v_witnesses").execute(&mut *conn).await?;

    let (fvk, _, _) = get_ivks(network, conn, account).await?;
    let notes = query(
        "SELECT
        a.id_note,
        a.diversifier,
        a.value,
        a.rcm,
        a.rho,
        a.scope,
        a.position,
        a.height
        FROM notes a
        LEFT JOIN spends b
        ON a.id_note = b.id_note AND b.height < ?1
        WHERE b.id_note IS NULL
        AND a.height < ?1
        AND a.account = ?2
        AND a.pool = 2",
    )
    .bind(height)
    .bind(account)
    .map(|r: SqliteRow| {
        let id: u32 = r.get(0);
        let diversifier: Vec<u8> = r.get(1);
        let value: u64 = r.get(2);
        let rcm: Vec<u8> = r.get(3);
        let rho: Vec<u8> = r.get(4);
        let scope: u32 = r.get(5);
        let position: u32 = r.get(6);
        let height: u32 = r.get(7);
        let note = note_from_parts(&fvk, scope, diversifier, rho, rcm, value);
        (id, note, position, scope, height)
    })
    .fetch_all(&mut *conn)
    .await?;

    for (id, note, position, scope, height) in notes.iter() {
        store_received_note(
            conn,
            domain,
            account,
            &fvk,
            Some(*id),
            note,
            &[],
            *height,
            *position,
            *scope,
        )
        .await?;
    }

    // Find first witness height after the snapshot
    let witness_height = query(
        "SELECT DISTINCT height FROM witnesses
        WHERE height >= ?1 AND account = ?2
        ORDER BY height LIMIT 1",
    )
    .bind(height)
    .bind(account)
    .map(|r: SqliteRow| r.get::<u32, _>(0))
    .fetch_optional(&mut *conn)
    .await
    .context("get witness_height")?;

    if let Some(witness_height) = witness_height {
        let tree_state = client
            .get_tree_state(Request::new(BlockId {
                height: height as u64,
                hash: vec![],
            }))
            .await?
            .into_inner();
        let orchard_tree = hex::decode(&tree_state.orchard_tree).anyhow()?;
        let orchard_tree = CommitmentTreeFrontier::read(&*orchard_tree).anyhow()?;
        let hasher = OrchardHasher::default();
        let edge_position = orchard_tree.to_edge(&hasher).to_auth_path(&hasher).1;

        for (id, note, _, _, _) in notes.iter() {
            let witness = query(
                "SELECT witness FROM witnesses
            WHERE account = ?1 AND note = ?2 AND height = ?3",
            )
            .bind(account)
            .bind(*id)
            .bind(witness_height)
            .map(|r: SqliteRow| {
                let witness: Vec<u8> = r.get(0);
                let (witness, _) =
                    bincode::decode_from_slice::<Witness, _>(&witness, legacy()).unwrap();
                witness
            })
            .fetch_one(&mut *conn)
            .await
            .with_context(|| format!("Cannot find witness {account} {} {witness_height}", *id))?;
            let cmx_proof = witness.rewind(edge_position);

            let nf = note.nullifier(&fvk);
            let nf = Fp::from_repr(nf.to_bytes()).unwrap();
            let nf_proof = pir_client.fetch_proof(nf).await?;

            let nf_proof: ImtProofDataBin = nf_proof.into();
            let nf_bytes = bincode::encode_to_vec(&nf_proof, legacy()).anyhow()?;

            let cmx_bytes = bincode::encode_to_vec(&cmx_proof, legacy()).anyhow()?;

            query(
                "INSERT INTO v_witnesses(id_note, nf, cmx)
            VALUES (?1, ?2, ?3)",
            )
            .bind(*id)
            .bind(&nf_bytes)
            .bind(&cmx_bytes)
            .execute(&mut *conn)
            .await?;
        }
    }

    Ok(())
}

pub async fn list_unspent_notes(
    conn: &mut SqliteConnection,
    id_account: u32,
) -> ZCVResult<Vec<UTXO>> {
    let utxos = query(
        "SELECT n.height, scope, position, nf, dnf, rho, diversifier, rseed, n.value
        FROM v_notes n LEFT JOIN v_spends s ON n.id_note = s.id_note
        WHERE s.id_note IS NULL
        AND n.account = ?1",
    )
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

pub async fn get_balance(conn: &mut SqliteConnection, id_account: u32) -> ZCVResult<u64> {
    let utxos = list_unspent_notes(conn, id_account).await?;
    let balance = utxos.iter().map(|utxo| utxo.value).sum::<u64>();
    Ok(balance)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::{
        balance::get_balance,
        tests::{get_connection, test_setup},
    };

    // disable for now
    // #[tokio::test]
    #[serial_test::serial]
    async fn test_question_balance() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        let balance = get_balance(&mut conn, 0).await?;
        assert_eq!(balance, 1169078);
        Ok(())
    }
}
