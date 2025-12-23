use ff::PrimeField;
use orchard::{
    Address,
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
    vote::{Ballot, BallotAnchors, BallotData, encrypt_ballot_action, try_decrypt_ballot},
};
use pasta_curves::Fp;
use rand_core::{CryptoRng, RngCore};
use sqlx::{SqliteConnection, query};
use zcash_protocol::consensus::Network;

use crate::{
    ZCVError, ZCVResult,
    balance::list_unspent_notes,
    db::{get_ivks, get_question, store_received_note},
    error::IntoAnyhow,
    tiu,
};

pub async fn encrypt_ballot_data<R: CryptoRng + RngCore>(
    network: &Network,
    conn: &mut SqliteConnection,
    domain: Fp,
    answer: usize,
    amount: u64,
    mut rng: R,
) -> ZCVResult<BallotData> {
    let (fvk, _, _) = get_ivks(network, conn).await?;
    let question = get_question(conn, domain).await?;
    let answer = &question.answers[answer];
    let (_, recipient) = bech32::decode(&answer.address).anyhow()?;
    let recipient = Address::from_raw_address_bytes(&tiu!(recipient)).unwrap();
    let mut a = 0u64;
    let utxos = list_unspent_notes(conn, domain).await?;
    let mut spends = vec![];
    for utxo in utxos {
        let u = a.min(utxo.value);
        a -= u;
        spends.push(utxo);
        if a == 0 {
            break;
        }
    }
    if a > 0 {
        return Err(ZCVError::NotEnoughVotes);
    }

    let mut actions = vec![];
    let mut a = amount;
    for spend in spends {
        let spend = spend.to_note(&fvk);
        let spend_amount = a.min(spend.value().inner());
        let (action, _, _) =
            encrypt_ballot_action(domain, fvk.clone(), &spend, recipient, a, &mut rng)?;
        a -= spend_amount;
        actions.push(action);
    }
    assert_eq!(a, 0);
    let ballot = BallotData {
        version: 1,
        domain: domain.to_repr().to_vec(),
        actions,
        anchors: BallotAnchors {
            nf: vec![],
            cmx: vec![],
        },
    };
    Ok(ballot)
}

#[allow(clippy::too_many_arguments)]
pub async fn decrypt_ballot_data(
    conn: &mut SqliteConnection,
    fvk: FullViewingKey,
    domain: Fp,
    question: u32,
    height: u32,
    position: u32,
    ballot: BallotData,
) -> ZCVResult<()> {
    let ivk = fvk.to_ivk(Scope::External);
    let ivk = PreparedIncomingViewingKey::new(&ivk);
    for (i, action) in ballot.actions.iter().enumerate() {
        if let Some(note) = try_decrypt_ballot(&ivk, action)? {
            store_received_note(
                conn,
                domain,
                &fvk,
                &note,
                height,
                position + i as u32,
                question,
                0, // ballots are sent to the external address
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use orchard::{
        keys::{FullViewingKey, PreparedIncomingViewingKey, Scope},
        vote::try_decrypt_ballot,
    };
    use rand_core::OsRng;
    use zcash_protocol::consensus::{MainNetwork, Network, NetworkConstants};

    use crate::{
        ballot::encrypt_ballot_data,
        db::get_domain,
        election::derive_question_sk,
        error::IntoAnyhow,
        tests::{TEST_ELECTION_SEED, get_connection, run_scan, test_setup},
    };

    #[tokio::test]
    #[serial_test::serial]
    async fn test_ballot_encryption() -> Result<()> {
        let mut conn = get_connection().await?;
        test_setup(&mut conn).await?;
        run_scan(&mut conn).await?;
        let domain = get_domain(&mut conn, 1, 2 /* question index */).await?;
        let ballot = encrypt_ballot_data(
            &Network::MainNetwork,
            &mut conn,
            domain,
            1, /* answer index */
            100000,
            OsRng,
        )
        .await?;
        let spk =
            derive_question_sk(TEST_ELECTION_SEED, MainNetwork.coin_type(), domain).anyhow()?;
        let fvk = FullViewingKey::from(&spk);
        let ivk = PreparedIncomingViewingKey::new(&fvk.to_ivk(Scope::External));
        let n = try_decrypt_ballot(&ivk, &ballot.actions[0])?;
        assert!(n.is_some());
        Ok(())
    }
}
