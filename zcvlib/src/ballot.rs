use orchard::{
    keys::PreparedIncomingViewingKey,
    vote::{BallotData, try_decrypt_ballot},
};
use pasta_curves::Fp;
use sqlx::SqliteConnection;
use zcash_protocol::consensus::Network;

use crate::{
    ZCVResult,
    db::{get_ivks, store_received_note},
};

pub async fn decrypt_ballot_data(
    network: &Network,
    conn: &mut SqliteConnection,
    domain: Fp,
    question: u32,
    height: u32,
    position: u32,
    ballot: BallotData,
) -> ZCVResult<()> {
    let (fvk, ivk, _) = get_ivks(network, conn).await?;
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
