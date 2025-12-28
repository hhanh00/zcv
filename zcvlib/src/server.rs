use crate::{
    ZCVError, ZCVResult,
    context::Context,
    db::{get_apphash, get_election, store_apphash, store_ballot},
    error::IntoAnyhow,
    server::rpc::submit_ballot,
    vote_rpc::VoteMessage,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use blake2b_simd::Params;
use parking_lot::Mutex;
use prost::Message;
use rocket::{figment::Figment, routes};
use rocket_cors::CorsOptions;
use serde_json::Value;
use sqlx::{Acquire, SqlitePool};
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

pub mod rpc;

pub type RPCResult<T> = Result<T, String>;

#[derive(Clone)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
}

impl Server {
    pub async fn new(pool: SqlitePool, domain: &[u8]) -> ZCVResult<Self> {
        let server = ServerState::new(pool, domain).await?;
        Ok(Self {
            state: Arc::new(Mutex::new(server)),
        })
    }
}

pub struct ServerState {
    pub pool: SqlitePool,
    pub domain: Vec<u8>,
    pub start_height: u32,
}

impl ServerState {
    pub async fn new(pool: SqlitePool, domain: &[u8]) -> ZCVResult<Self> {
        let mut conn = pool.acquire().await?;
        let e = get_election(&mut conn, domain).await?;

        Ok(Self {
            pool,
            domain: domain.to_vec(),
            start_height: e.end,
        })
    }

    pub fn check_ballot(&mut self, ballot: orchard::vote::Ballot) -> ZCVResult<()> {
        tracing::info!("check_ballot {:?}", &ballot);
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
                let ballot: orchard::vote::Ballot = serde_json::from_str(&ballot.ballot)?;
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
        let RequestFinalizeBlock {
            txs, hash, height, ..
        } = request;
        tracing::info!("Hash {} height {height}", hex::encode(&hash));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let state = self.state.lock();
        let tx_results = rt
            .block_on(async move {
                let mut conn = state.pool.acquire().await?;
                let mut db_tx = conn.begin().await?;
                let apphash = get_apphash(&mut db_tx, &state.domain).await?;
                let mut hasher = Params::new()
                    .personal(b"ZCVote___AppHash")
                    .hash_length(32)
                    .key(&apphash)
                    .to_state();
                let mut tx_results = vec![];
                for (itx, mut tx) in txs.into_iter().enumerate() {
                    let tx_copy = tx.clone();
                    hasher.update(&tx_copy);
                    let finalize = async {
                        let msg = VoteMessage::decode(&mut tx)?;
                        if let Some(m) = msg.type_oneof
                            && let crate::vote_rpc::vote_message::TypeOneof::Ballot(ballot) = m
                        {
                            let ballot: orchard::vote::Ballot =
                                serde_json::from_str(&ballot.ballot)?;
                            store_ballot(
                                &mut db_tx,
                                state.start_height + height as u32,
                                itx as u32,
                                ballot,
                            )
                            .await?;
                        }
                        Ok::<_, ZCVError>(())
                    };
                    let result = match finalize.await {
                        Ok(_) => ExecTxResult {
                            code: 0,
                            ..ExecTxResult::default()
                        },
                        Err(error) => ExecTxResult {
                            code: 1,
                            data: tx_copy,
                            log: error.to_string(),
                            info: "Error in finalization".to_string(),
                            ..ExecTxResult::default()
                        },
                    };
                    tx_results.push(result);
                }
                let apphash = hasher.finalize().as_bytes().to_vec();
                store_apphash(&mut db_tx, &state.domain, &apphash).await?;

                db_tx.commit().await?;
                Ok::<_, ZCVError>(tx_results)
            })
            .expect("Fatal Failure in FinalizeBlock");

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

pub async fn run_cometbft_app(context: &Context, domain: &[u8], port: u16) -> ZCVResult<()> {
    let app = Server::new(context.pool.clone(), domain).await?;
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
