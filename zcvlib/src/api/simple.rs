use anyhow::Result;
use flutter_rust_bridge::frb;
use zcash_protocol::consensus::Network;

use crate::api::Context;
use crate::db::get_election;
use crate::pod::{ElectionProps, ElectionPropsPub};
use crate::lwd::connect;

#[frb(sync)]
pub fn compile_election_def(election_json: String, seed: String) -> Result<String> {
    let election: ElectionProps = serde_json::from_str(&election_json)?;
    let epub = election.build(&seed)?;
    let res = serde_json::to_string(&epub).unwrap();
    Ok(res)
}

#[frb]
pub async fn store_election(election_json: String, context: &Context) -> Result<Vec<u8>> {
    let mut conn = context.connect().await?;
    let election: ElectionPropsPub = serde_json::from_str(&election_json)?;
    crate::db::store_election(&mut conn, &election).await?;
    Ok(election.hash()?.to_vec())
}

pub async fn scan_notes(hash: String, id_account: u32, context: &Context) -> Result<()> {
    let hash = hex::decode(&hash)?;
    let mut conn = context.connect().await?;
    let e = get_election(&mut conn, &hash).await?;
    let mut client = connect(&context.lwd_url).await?;
    crate::lwd::scan_blocks(
        &Network::MainNetwork,
        &mut conn,
        &mut client,
        &hash,
        id_account,
        e.start,
        e.end,
    )
    .await?;
    Ok(())
}
