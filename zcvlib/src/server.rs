use base64::{Engine, prelude::BASE64_STANDARD};
use parking_lot::Mutex;
use prost::Message;
use rocket::{figment::Figment, routes};
use rocket_cors::CorsOptions;
use serde_json::Value;
use sqlx::{SqliteConnection, query_as};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};
use tendermint_abci::{Application, ServerBuilder};
use tendermint_proto::abci::{
    ExecTxResult, RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestPrepareProposal,
    ResponseCheckTx, ResponseFinalizeBlock, ResponseInfo, ResponsePrepareProposal,
};
use crate::{
    ZCVError, ZCVResult, context::Context, db::get_election, error::IntoAnyhow, server::rpc::submit_ballot, vote_rpc::{Ballot, VoteMessage}
};

pub mod rpc;

pub type RPCResult<T> = Result<T, String>;

#[derive(Clone)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
}

impl Server {
    pub async fn new(conn: SqliteConnection, id_election: u32) -> ZCVResult<Self> {
        let server = ServerState::new(conn, id_election).await?;
        Ok(Self {
            state: Arc::new(Mutex::new(server)),
        })
    }
}

pub struct ServerState {
    pub conn: SqliteConnection,
    pub height: u32,
}

impl ServerState {
    pub async fn new(mut conn: SqliteConnection, id_election: u32) -> ZCVResult<Self> {
        let e = get_election(&mut conn, id_election).await?;
        let (h,): (Option<u32>,) = query_as("SELECT MAX(height) FROM ballots")
            .fetch_one(&mut conn)
            .await?;
        let height = h.unwrap_or(e.end) + 1;

        Ok(Self { conn, height })
    }

    pub fn check_ballot(&mut self, ballot: Ballot) -> ZCVResult<()> {
        let b: orchard::vote::Ballot = serde_json::from_str(&ballot.ballot)?;
        tracing::info!("check_ballot {}", serde_json::to_string(&b.data).unwrap());
        // TODO: Verify ballot signature & zkp, no need to verify double-spend yet
        Ok(())
    }
}

impl rocket::response::Responder<'_, 'static> for ZCVError {
    fn respond_to(self, request: &'_ rocket::Request<'_>) -> rocket::response::Result<'static> {
        let error_string = self.to_string();
        (rocket::http::Status::InternalServerError, error_string).respond_to(request)
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
    fn finalize_block(&self, request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        let rt = tokio::runtime::Runtime::new().unwrap();
        // Store ballot in db, etc.
        // rt.block_on(store_ballot(&mut self.conn, self.height, b))?;
        let tx_results: Vec<_> = request
            .txs
            .into_iter()
            .map(|tx| ExecTxResult {
                code: 0,
                data: tx,
                ..ExecTxResult::default()
            })
            .collect();
        ResponseFinalizeBlock {
            tx_results,
            ..ResponseFinalizeBlock::default()
        }
    }
}

pub async fn submit_tx(tx_bytes: &[u8], port: u16) -> ZCVResult<Value> {
    let tx_data = BASE64_STANDARD.encode(tx_bytes);
    let req_body = serde_json::json!({
        "id": "",
        "method": "broadcast_tx_sync",
        "params": [tx_data]
    });
    tracing::info!("submit_tx");
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

pub async fn run_cometbft_app(context: &Context, id_election: u32, port: u16) -> ZCVResult<()> {
    let conn = context.connect().await?.detach();
    let app = Server::new(conn, id_election).await?;
    let server = ServerBuilder::new(1_000_000)
        .bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port), app)
        .anyhow()?;
    server.listen().anyhow()?;
    Ok(())
}

pub fn run_rocket_server(config: Figment, mut context: Context, comet_port: u16) -> ZCVResult<()> {
    context.comet_port = comet_port;
    rocket::execute(async move {
        let cors = CorsOptions::default().to_cors().unwrap();
        let _rocket = rocket::custom(config)
            .attach(cors)
            .manage(context)
            .mount("/", routes![submit_ballot,])
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
