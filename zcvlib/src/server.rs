use base64::{Engine, prelude::BASE64_STANDARD};
use parking_lot::Mutex;
use prost::Message;
use rocket::{figment::Figment, routes};
use rocket_cors::CorsOptions;
use serde_json::Value;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};
use tendermint_abci::{Application, ServerBuilder};
use tendermint_proto::abci::{
    RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestPrepareProposal, ResponseCheckTx,
    ResponseFinalizeBlock, ResponseInfo, ResponsePrepareProposal,
};

use crate::{
    ZCVError, ZCVResult,
    context::Context,
    error::IntoAnyhow,
    vote_rpc::{Ballot, VoteMessage},
};

#[derive(Clone)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ServerState::new())),
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ServerState {}

impl ServerState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn check_ballot(&mut self, ballot: Ballot) -> ZCVResult<()> {
        let _b: orchard::vote::Ballot = serde_json::from_str(&ballot.ballot)?;
        // TODO: Verify ballot signature & zkp, no need to verify double-spend yet
        Ok(())
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for Server {
    fn info(&self, _request: RequestInfo) -> ResponseInfo {
        Default::default()
    }

    // Checks if a tx is structurally correct
    // Valid txs must not be rejected
    // But bad txs may be kept for the moment
    fn check_tx(&self, request: RequestCheckTx) -> ResponseCheckTx {
        let RequestCheckTx { mut tx, .. } = request;
        let mut check = || {
            let msg = VoteMessage::decode(&mut tx)?;
            if let Some(m) = msg.type_oneof
                && let crate::vote_rpc::vote_message::TypeOneof::Ballot(ballot) = m
            {
                let mut state = self.state.lock();
                state.check_ballot(ballot)?;
            }
            Ok::<_, ZCVError>(())
        };

        if let Err(err) = check() {
            return ResponseCheckTx {
                code: 1,
                data: tx,
                log: err.to_string(),
                ..Default::default()
            };
        }
        Default::default()
    }

    // Select txs to include in a block
    // Bad txs must be rejected
    // Valid txs may be excluded (for example, if the block is full)
    fn prepare_proposal(&self, request: RequestPrepareProposal) -> ResponsePrepareProposal {
        // Per the ABCI++ spec: if the size of RequestPrepareProposal.txs is
        // greater than RequestPrepareProposal.max_tx_bytes, the Application
        // MUST remove transactions to ensure that the
        // RequestPrepareProposal.max_tx_bytes limit is respected by those
        // transactions returned in ResponsePrepareProposal.txs.
        let RequestPrepareProposal {
            mut txs,
            max_tx_bytes,
            ..
        } = request;
        let max_tx_bytes: usize = max_tx_bytes.try_into().unwrap_or(0);
        let mut total_tx_bytes: usize = txs
            .iter()
            .map(|tx| tx.len())
            .fold(0, |acc, len| acc.saturating_add(len));
        while total_tx_bytes > max_tx_bytes {
            if let Some(tx) = txs.pop() {
                total_tx_bytes = total_tx_bytes.saturating_sub(tx.len());
            } else {
                break;
            }
        }
        ResponsePrepareProposal { txs }
    }

    // Process the block that was voted on by the validators
    fn finalize_block(&self, _request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        Default::default()
    }
}

pub async fn submit_tx(tx_bytes: &[u8], port: u16) -> ZCVResult<Value> {
    let tx_data = BASE64_STANDARD.encode(tx_bytes);
    let req_body = serde_json::json!({
        "id": "",
        "method": "broadcast_tx_sync",
        "params": [tx_data]
    });
    let url = format!("http://127.0.0.1:{port}/v1");
    let client = reqwest::Client::new();
    let rep = client
        .post(&url)
        .timeout(Duration::from_secs(300))
        .json(&req_body)
        .send()
        .await?
        .error_for_status()?;
    let json_rep: Value = rep.json().await?;
    Ok(json_rep)
}

pub fn run_cometbft_app(port: u16) -> ZCVResult<()> {
    let app = Server::new();
    let server = ServerBuilder::new(1_000_000)
        .bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port), app)
        .anyhow()?;
    server.listen().anyhow()?;
    Ok(())
}

pub fn run_rocket_server(config: Figment, context: Context) -> ZCVResult<()> {
    rocket::execute(async move {
        let cors = CorsOptions::default().to_cors().unwrap();
        let _rocket = rocket::custom(config)
            .attach(cors)
            .manage(context)
            .mount("/", routes![])
            .ignite()
            .await
            .anyhow()?
            .launch()
            .await
            .anyhow()?;
        Ok::<_, ZCVError>(())
    })?;
    Ok(())
}
