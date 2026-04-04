use anyhow::Result;
use ff::PrimeField;
use orchard::vote::Ballot;
use pasta_curves::Fp;
use pir_client::PirClient;
use tonic::Request;
use tonic::transport::Endpoint;
use zcash_protocol::consensus::Network;

use crate::context::Context;
use crate::db::{get_election, get_election_height};
use crate::lwd::{VoteClient, connect};
use crate::pod::{ElectionProps, ElectionPropsPub};
use crate::tiu;
use crate::vote::VoteResultItem;
use crate::vote_rpc::Empty;
use crate::vote_rpc::vote_streamer_client::VoteStreamerClient;

pub fn compile_election_def(election_json: String, seed: String) -> Result<String> {
    let election: ElectionProps = serde_json::from_str(&election_json)?;
    let epub = election.build(&seed)?;
    let res = serde_json::to_string(&epub).unwrap();
    Ok(res)
}

pub async fn store_election(election_json: String, context: &Context) -> Result<Vec<u8>> {
    let mut conn = context.connect().await?;
    let election: ElectionPropsPub = serde_json::from_str(&election_json)?;
    crate::db::store_election(&mut conn, &election, &[], &[]).await?;
    Ok(election.domain.clone())
}

pub async fn client_delete_election(context: &Context) -> Result<()> {
    let mut conn = context.connect().await?;
    crate::db::client_delete_election(&mut conn).await?;
    Ok(())
}

pub async fn client_delete_election_data(context: &Context, new_account: Option<u32>) -> Result<()> {
    let mut conn = context.connect().await?;
    crate::db::client_delete_election_data(&mut conn, new_account).await?;
    Ok(())
}

pub async fn scan_ballots(id_account: u32, context: &Context) -> Result<()> {
    let mut conn = context.connect().await?;
    let ep = Endpoint::from_shared(context.election_url.clone())?;
    let mut client = VoteStreamerClient::connect(ep).await?;
    let pir_client = PirClient::connect(&context.pir_url).await?;
    let (election, ..) = get_election(&mut conn).await?;
    let domain = Fp::from_repr(tiu!(election.domain)).unwrap();
    let start = get_election_height(&mut conn).await? + 1;
    let rep = client
        .get_latest_vote_height(Request::new(Empty {}))
        .await?;
    let end = rep.into_inner().height;
    crate::lwd::scan_ballots(
        &Network::MainNetwork,
        &mut conn,
        &mut client,
        &pir_client,
        domain,
        id_account,
        start,
        end,
    )
    .await?;
    Ok(())
}

pub async fn decode_ballots(election_seed: String, context: &Context) -> Result<()> {
    let mut conn = context.connect().await?;
    let ep = Endpoint::from_shared(context.election_url.clone())?;
    let mut client = VoteStreamerClient::connect(ep).await?;
    let election = client.get_election(Request::new(Empty {})).await?.into_inner();
    let election: ElectionPropsPub = serde_json::from_str(&election.election)?;
    let start = election.end;
    let rep = client
        .get_latest_vote_height(Request::new(Empty {}))
        .await?;
    let end = rep.into_inner().height;
    crate::lwd::decode_ballots(
        &Network::MainNetwork,
        &mut conn,
        &mut client,
        &election_seed,
        start,
        end,
    )
    .await?;
    Ok(())
}

pub async fn collect_results(context: &Context) -> Result<Vec<VoteResultItem>> {
    let mut conn = context.connect().await?;
    let res = crate::vote::collect_results(&mut conn).await?;
    Ok(res)
}

pub async fn get_balance(
    id_account: u32,
    context: &Context,
) -> Result<u64> {
    let mut conn = context.connect().await?;
    let balance = crate::balance::get_balance(&mut conn, id_account).await?;
    Ok(balance)
}

async fn submit_ballot(ballot: Ballot, context: &Context) -> Result<Vec<u8>> {
    let mut ballot_bytes = vec![];
    ballot.write(&mut ballot_bytes)?;
    let mut client = connect_to_vote_server(context).await?;
    client
        .submit_vote(Request::new(crate::vote_rpc::Ballot {
            ballot: ballot_bytes,
            ..Default::default()
        }))
        .await?;
    let txid = ballot.data.sighash()?;
    Ok(txid)
}

pub async fn vote(
    id_account: u32,
    vote_content: String,
    amount: u64,
    context: &Context,
) -> Result<Vec<u8>> {
    let memo = hex::decode(&vote_content)?;
    let mut conn = context.connect().await?;
    let ballot = crate::vote::vote(
        &Network::MainNetwork,
        &mut conn,
        &context.lwd_url,
        &context.pir_url,
        id_account,
        &memo,
        amount,
    )
    .await?;
    let txid = submit_ballot(ballot, context).await?;
    Ok(txid)
}

pub async fn mint(
    id_account: u32,
    amount: u64,
    context: &Context,
) -> Result<()> {
    let mut conn = context.connect().await?;
    let ballot = crate::vote::mint(
        &Network::MainNetwork,
        &mut conn,
        id_account,
        amount,
    )
    .await?;
    submit_ballot(ballot, context).await?;
    Ok(())
}

pub async fn delegate(
    id_account: u32,
    address: &str,
    amount: u64,
    context: &Context,
) -> Result<Vec<u8>> {
    let mut conn = context.connect().await?;
    let ballot = crate::vote::delegate(
        &Network::MainNetwork,
        &mut conn,
        &context.lwd_url,
        &context.pir_url,
        id_account,
        address,
        amount,
    )
    .await?;
    let txid = submit_ballot(ballot, context).await?;
    Ok(txid)
}

pub async fn get_account_address(id_account: u32, context: &Context) -> Result<String> {
    let mut conn = context.connect().await?;
    let address =
        crate::db::get_account_address(&Network::MainNetwork, &mut conn, id_account).await?;
    Ok(address)
}

pub async fn import_election(context: &Context) -> Result<(Vec<u8>, Vec<u8>)>
{
    let mut client = connect_to_vote_server(context).await?;
    let election_json = client
        .get_election(Request::new(Empty {}))
        .await?
        .into_inner()
        .election;
    let election: crate::pod::ElectionPropsPub = serde_json::from_str(&election_json)?;
    let (nf_root, cmx_tree) = crate::lwd::fetch_roots(&context.lwd_url, &context.pir_url, election.end).await?;
    let mut conn = context.connect().await?;
    crate::db::store_election(&mut conn, &election, &nf_root, &cmx_tree).await?;
    Ok((nf_root, cmx_tree))
}

pub async fn import_account(id_account: u32, context: &Context) -> Result<()> {
    let mut conn = context.connect().await?;
    let mut client = connect(&context.lwd_url).await?;
    let pir_client = PirClient::connect(&context.pir_url).await?;
    let (election, ..) = get_election(&mut conn).await?;
    let domain = Fp::from_repr(tiu!(election.domain)).unwrap();
    let height = election.end;
    crate::balance::import_account(&Network::MainNetwork,
        &mut conn,
        &mut client, &pir_client,
        id_account, domain, height).await?;
    Ok(())
}

async fn connect_to_vote_server(context: &Context) -> Result<VoteClient> {
    let ep = Endpoint::from_shared(context.election_url.clone())?;
    let client = VoteStreamerClient::connect(ep).await?;
    Ok(client)
}
