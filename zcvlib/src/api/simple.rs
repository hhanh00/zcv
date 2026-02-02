use anyhow::Result;
use flutter_rust_bridge::frb;
use orchard::vote::Ballot;
use tonic::Request;
use tonic::transport::Endpoint;
use zcash_protocol::consensus::Network;

use crate::api::Context;
use crate::db::{get_domain, get_election};
use crate::pod::{ElectionProps, ElectionPropsPub};
use crate::lwd::connect;
use crate::vote_rpc::vote_streamer_client::VoteStreamerClient;

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

#[frb]
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

#[frb]
pub async fn get_balance(hash: String, id_account: u32, idx_question: u32, context: &Context) -> Result<u64> {
    let mut conn = context.connect().await?;
    let hash = hex::decode(&hash)?;
    let (domain, _) = get_domain(&mut conn, &hash, idx_question as usize).await?;
    let balance = crate::balance::get_balance(&mut conn, domain, id_account).await?;
    Ok(balance)
}

pub async fn vote(hash: String, id_account: u32, idx_question: u32, vote_content: String, amount: u64, context: &Context) -> Result<()> {
    let hash = hex::decode(&hash)?;
    let memo = hex::decode(&vote_content)?;
    let mut conn = context.connect().await?;
    let Ballot { data, witnesses } = crate::vote::vote(&Network::MainNetwork, &mut conn, &hash, id_account, idx_question, &memo, amount).await?;
    let mut data_bytes = vec![];
    data.write(&mut data_bytes)?;
    let mut witnesses_bytes = vec![];
    witnesses.write(&mut witnesses_bytes)?;
    let ep = Endpoint::from_shared(context.election_url.clone())?;
    let mut client = VoteStreamerClient::connect(ep).await?;

    client.submit_vote(Request::new(crate::vote_rpc::Ballot {
        data: data_bytes,
        witnesses: witnesses_bytes,
        ..Default::default()
    })).await?;
    Ok(())
}
