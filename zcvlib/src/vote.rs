use std::{collections::HashMap, sync::LazyLock};

use bincode::config::legacy;
use ff::PrimeField;
use orchard::{
    Address, Note,
    vote::{
        Ballot, BallotWitnesses, Circuit, CmxInclusion, MerklePathGeneric,
        NfExclusion, ProvingKey, VerifyingKey, vote_with_nf_exclusion,
    },
};
use pasta_curves::Fp;
use pir_client::ImtProofData;
use rand_core::OsRng;
use sqlx::{Row, SqliteConnection, query, sqlite::SqliteRow};
use zcash_protocol::consensus::Network;
use zcash_trees::warp::{
    AuthPath, FragmentAuthPath, Witness,
    hasher::{OrchardHasher, empty_roots},
    legacy::CommitmentTreeFrontier,
};

use crate::{
    ZCVResult,
    balance::list_unspent_notes,
    ballot::encrypt_ballot_data_with_spends,
    db::{get_account_address, get_account_sk, get_domain, get_election, get_ivks},
    error::IntoAnyhow,
    lwd::fetch_roots,
    pod::ImtProofDataBin,
    tiu,
};

pub async fn vote(
    network: &Network,
    conn: &mut SqliteConnection,
    lwd_url: &str,
    pir_url: &str,
    id_account: u32,
    memo: &[u8],
    amount: u64,
) -> ZCVResult<Ballot> {
    let (domain, address) = get_domain(conn).await?;
    send_vote(network, conn, lwd_url, pir_url, id_account, domain, &address, memo, amount).await
}

#[allow(clippy::too_many_arguments)]
pub async fn send_vote(
    network: &Network,
    conn: &mut SqliteConnection,
    lwd_url: &str,
    pir_url: &str,
    id_account: u32,
    domain: Fp,
    address: &str,
    memo: &[u8],
    amount: u64,
) -> ZCVResult<Ballot> {
    tracing::info!("send_vote");
    let (_, recipient) = bech32::decode(address).unwrap();
    let recipient = Address::from_raw_address_bytes(&tiu!(recipient)).unwrap();

    tracing::info!("get_election");
    let (e, ..) = get_election(conn).await?;
    let sk = if e.need_sig {
        Some(get_account_sk(network, conn, id_account).await?)
    } else {
        None
    };
    tracing::info!("get_ivks");
    let (fvk, _, _) = get_ivks(network, conn, id_account).await?;
    tracing::info!("list_unspent_notes");
    let utxos = list_unspent_notes(conn, id_account).await?;
    let notes = utxos
        .into_iter()
        .map(|utxo| {
            let p = utxo.position;
            let n = utxo.to_note(&fvk);
            (n, p)
        })
        .collect::<Vec<_>>();

    tracing::info!("fetch_roots");
    // Fetch the stored nf_root and CMX commitment tree frontier from the DB.
    let (nf_root_bytes, cmx_tree_bytes) = fetch_roots(lwd_url, pir_url, e.end).await?;
    let nf_root = Fp::from_repr(tiu!(nf_root_bytes)).unwrap();

    tracing::info!("cmx_frontier");
    // Build the CMX edge (FragmentAuthPath) used to complete witnesses.
    let cmx_frontier = CommitmentTreeFrontier::read(&*cmx_tree_bytes).anyhow()?;
    let hasher = OrchardHasher::default();
    let er = empty_roots(&hasher);
    let cmx_edge = cmx_frontier.to_edge(&hasher);
    let edge = cmx_edge.to_auth_path(&hasher);
    let cmx_root = Fp::from_repr(cmx_edge.root(&hasher)).unwrap();

    tracing::info!("note_ids");
    // Fetch the id_note for each unspent note (same filter as list_unspent_notes).
    let note_ids: Vec<u32> = query(
        "SELECT n.id_note FROM v_notes n LEFT JOIN v_spends s ON n.id_note = s.id_note
        WHERE s.id_note IS NULL AND n.account = ?1",
    )
    .bind(id_account)
    .map(|r: SqliteRow| r.get::<u32, _>(0))
    .fetch_all(&mut *conn)
    .await?;

    tracing::info!("nf_witnesses");
    // Build per-note NF exclusion and CMX inclusion proofs via the stored witnesses,
    // bundled with each note so selection never goes out of sync.
    let mut notes_with_witnesses: Vec<(Note, u32, NfExclusion, CmxInclusion)> =
        Vec::with_capacity(note_ids.len());

    for (id_note, (note, pos)) in note_ids.iter().zip(notes.into_iter()) {
        let (nf_excl, cmx_incl) = get_merkle_proofs(conn, *id_note, &edge, &er).await?;
        notes_with_witnesses.push((note, pos, nf_excl, cmx_incl));
    }

    tracing::info!("vote_with_nf_exclusion");
    let (ballot, _) = vote_with_nf_exclusion(
        domain,
        e.need_sig,
        sk,
        &fvk,
        recipient,
        amount,
        memo,
        &notes_with_witnesses,
        nf_root,
        cmx_root,
        OsRng,
        |message, _, _| {
            tracing::info!("{}", message);
        },
        &PK,
        &VK,
    )?;

    Ok(ballot)
}

pub async fn get_merkle_proofs(
    conn: &mut SqliteConnection,
    id_note: u32,
    edge: &FragmentAuthPath,
    empty_roots: &AuthPath,
) -> ZCVResult<(NfExclusion, CmxInclusion)> {
    let (nf_bytes, cmx_bytes) = query("SELECT nf, cmx FROM v_witnesses WHERE id_note = ?1")
        .bind(id_note)
        .map(|r: SqliteRow| {
            let nf: Vec<u8> = r.get(0);
            let cmx: Vec<u8> = r.get(1);
            (nf, cmx)
        })
        .fetch_one(&mut *conn)
        .await?;

    let (nf_bin, _) =
        bincode::decode_from_slice::<ImtProofDataBin, _>(&nf_bytes, legacy()).unwrap();
    let nf: ImtProofData = nf_bin.into();
    let nf_exclusion = NfExclusion {
        nf_width: nf.width,
        nf_path: MerklePathGeneric::from_parts(nf.low, nf.leaf_pos, nf.path),
    };

    let (cmx, _) = bincode::decode_from_slice::<Witness, _>(&cmx_bytes, legacy()).unwrap();
    let auth_path = cmx.build_auth_path(edge, empty_roots)?;
    let value = Fp::from_repr(cmx.value).unwrap();
    let path: [Fp; 32] = auth_path.0.map(|h| Fp::from_repr(h).unwrap());
    let cmx_inclusion = CmxInclusion {
        cmx_path: MerklePathGeneric::from_parts(value, cmx.position, path),
    };

    Ok((nf_exclusion, cmx_inclusion))
}

pub fn expand_into_ranges(nfs: Vec<Fp>) -> Vec<Fp> {
    let mut prev = Fp::zero();
    let mut nf_ranges = vec![];
    for r in nfs {
        // Skip empty ranges when nfs are consecutive
        // (with statistically negligible odds)
        if prev < r {
            // Ranges are inclusive of both ends
            let a = prev;
            let b = r - Fp::one();

            nf_ranges.push(a);
            nf_ranges.push(b);
        }
        prev = r + Fp::one();
    }
    let a = prev;
    let b = Fp::one().neg();

    nf_ranges.push(a);
    nf_ranges.push(b);
    nf_ranges
}

pub async fn mint(
    network: &Network,
    conn: &mut SqliteConnection,
    id_account: u32,
    amount: u64,
) -> ZCVResult<Ballot> {
    let (domain, _) = get_domain(conn).await?;
    let address = get_account_address(network, conn, id_account).await?;

    let data = encrypt_ballot_data_with_spends(
        network,
        conn,
        domain,
        id_account,
        &address,
        &[],
        amount,
        vec![],
        amount,
        OsRng,
    )
    .await?;
    Ok(Ballot {
        data,
        witnesses: dummy_witnesses(),
    })
}

pub async fn delegate(
    network: &Network,
    conn: &mut SqliteConnection,
    lwd_url: &str,
    pir_url: &str,
    id_account: u32,
    address: &str,
    amount: u64,
) -> ZCVResult<Ballot> {
    let (domain, _) = get_domain(conn).await?;
    send_vote(
        network,
        conn,
        lwd_url,
        pir_url,
        id_account,
        domain,
        address,
        &[],
        amount,
    )
    .await
}

fn dummy_witnesses() -> BallotWitnesses {
    BallotWitnesses {
        proofs: vec![],
        sp_signatures: None,
        binding_signature: [0u8; 64],
    }
}

pub async fn collect_results(conn: &mut SqliteConnection) -> ZCVResult<Vec<VoteResultItem>> {
    query("DELETE FROM v_final_results")
        .execute(&mut *conn)
        .await?;
    let results = query("SELECT answer, votes FROM v_results")
        .map(|r: SqliteRow| {
            let answer: Vec<u8> = r.get(0);
            let votes: u64 = r.get(1);
            (answer, votes)
        })
        .fetch_all(&mut *conn)
        .await?;
    let mut items: HashMap<VoteResultItem, u64> = HashMap::new();
    for (answer, votes) in results {
        for (i, a) in answer.iter().enumerate() {
            if *a == 0 {
                break;
            }
            let item = VoteResultItem {
                idx_question: i as u32,
                idx_answer: *a,
                votes: 0,
            };
            let e = items.entry(item).or_default();
            *e += votes;
        }
    }
    for (k, v) in items {
        query(
            "INSERT INTO v_final_results
        (idx_question, idx_answer, votes)
        VALUES (?1, ?2, ?3)",
        )
        .bind(k.idx_question)
        .bind(k.idx_answer)
        .bind(v as i64)
        .execute(&mut *conn)
        .await?;
    }
    let counts = query(
        "SELECT idx_question, idx_answer, votes
    FROM v_final_results ORDER BY idx_question, idx_answer",
    )
    .map(|r: SqliteRow| {
        let idx_question: u32 = r.get(0);
        let idx_answer: u8 = r.get(1);
        let votes: u64 = r.get(2);
        VoteResultItem {
            idx_question,
            idx_answer,
            votes,
        }
    })
    .fetch_all(&mut *conn)
    .await?;
    Ok(counts)
}

#[derive(Hash, PartialEq, Eq)]
pub struct VoteResultItem {
    pub idx_question: u32,
    pub idx_answer: u8,
    pub votes: u64,
}

pub static PK: LazyLock<ProvingKey<Circuit>> = LazyLock::new(ProvingKey::build);
pub static VK: LazyLock<VerifyingKey<Circuit>> = LazyLock::new(VerifyingKey::build);

#[cfg(test)]
mod tests {
    use crate::{
        db::get_domain,
        tests::{get_connection, test_setup},
    };
    use anyhow::Result;

    #[tokio::test]
    #[serial_test::serial]
    async fn test_vote() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        let (_domain, _address) = get_domain(&mut conn).await?;

        // TODO
        Ok(())
    }
}
