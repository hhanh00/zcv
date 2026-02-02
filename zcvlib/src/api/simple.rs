use anyhow::Result;
use flutter_rust_bridge::frb;

use crate::api::Context;
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
pub async fn store_election(election_json: String, context: &Context) -> Result<u32> {
    tracing::info!("{election_json}");
    let mut conn = context.connect().await?;
    let election: ElectionPropsPub = serde_json::from_str(&election_json)?;
    let id = crate::db::store_election(&mut conn, &election).await?;
    Ok(id)
}

pub async fn scan_notes(context: &Context) -> Result<()> {
    let mut _conn = context.connect().await?;
    let mut _client = connect(&context.lwd_url).await?;
    // scan_blocks(
    //     &Network::MainNetwork,
    //     &mut conn,
    //     &mut client,
    //     1,
    //     3_168_000,
    //     3_169_000,
    // )
    // .await?;
    Ok(())
}
