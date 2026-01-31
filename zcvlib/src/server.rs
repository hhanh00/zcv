use crate::{
    ZCVError, ZCVResult,
    context::Context,
    db::{get_apphash, get_election, store_apphash, store_ballot},
    error::IntoAnyhow,
    vote_rpc::{Validator, VoteMessage, vote_message::TypeOneof},
};
use anyhow::anyhow;
use base64::{Engine, prelude::BASE64_STANDARD};
use blake2b_simd::Params;
use parking_lot::Mutex;
use prost::Message;
use serde_json::{Value, json};
use sqlx::{Acquire, SqlitePool, query, query_as};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};
use tendermint_abci::{Application, ServerBuilder};
use tendermint_proto::{
    abci::{
        ExecTxResult, RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestPrepareProposal,
        ResponseCheckTx, ResponseFinalizeBlock, ResponseInfo, ResponsePrepareProposal,
        ValidatorUpdate,
    },
    crypto::{PublicKey, public_key::Sum},
};

pub mod rpc;

pub type RPCResult<T> = Result<T, String>;

#[derive(Clone)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
}

impl Server {
    pub async fn new(pool: SqlitePool, hash: &[u8]) -> ZCVResult<Self> {
        let server = ServerState::new(pool, hash).await?;
        Ok(Self {
            state: Arc::new(Mutex::new(server)),
        })
    }
}

pub struct ServerState {
    pub pool: SqlitePool,
    pub hash: Vec<u8>,
    pub start_height: u32,
    pub started: bool,
}

impl ServerState {
    pub async fn new(pool: SqlitePool, hash: &[u8]) -> ZCVResult<Self> {
        let mut conn = pool.acquire().await?;
        let e = get_election(&mut conn, hash).await?;
        let (started,): (bool,) = query_as("SELECT started FROM state WHERE id = 0")
            .fetch_one(&mut *conn)
            .await?;

        Ok(Self {
            pool,
            hash: hash.to_vec(),
            start_height: e.end,
            started,
        })
    }

    pub fn check_ballot(&mut self, ballot: orchard::vote::Ballot) -> ZCVResult<()> {
        tracing::info!("check_ballot {:?}", &ballot);
        // TODO: Verify ballot signature & zkp, no need to verify double-spend yet
        Ok(())
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
            if let Some(m) = msg.type_oneof {
                match m {
                    TypeOneof::Ballot(ballot) => {
                        let mut state = self.state.lock();
                        let ballot = orchard::vote::Ballot::read(&*ballot.data).anyhow()?;
                        state.check_ballot(ballot)?;
                    }
                    TypeOneof::AddValidator(_) => {
                        let state = self.state.lock();
                        if state.started {
                            return Err(ZCVError::Any(anyhow!(
                                "Validators cannot be added once the blockchain is started."
                            )));
                        }
                    }
                    TypeOneof::Start(_) => {
                        let state = self.state.lock();
                        if state.started {
                            return Err(ZCVError::Any(anyhow!("Blockchain already started.")));
                        }
                    }
                    _ => {}
                }
            }
            Ok::<_, ZCVError>(())
        };

        if let Err(err) = check() {
            tracing::info!("check_tx: {}", err.to_string());
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
        let mut state = self.state.lock();
        let (tx_results, validator_updates) = rt
            .block_on(async move {
                let mut validator_updates = vec![];
                let mut conn = state.pool.acquire().await?;
                let mut db_tx = conn.begin().await?;
                let apphash = get_apphash(&mut db_tx, &state.hash).await?;
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
                        if let Some(m) = msg.type_oneof {
                            match m {
                                TypeOneof::Ballot(ballot) => {
                                    let ballot =
                                        orchard::vote::Ballot::read(&*ballot.data).anyhow()?;
                                    let rows_added = store_ballot(
                                        &mut db_tx,
                                        state.start_height + height as u32,
                                        itx as u32,
                                        ballot,
                                    )
                                    .await?;
                                    assert_eq!(rows_added, 1);
                                }
                                TypeOneof::Start(_) => {
                                    state.started = true;
                                    query("UPDATE state SET started = 1 WHERE id = 0")
                                        .execute(&mut *db_tx)
                                        .await?;
                                }
                                TypeOneof::AddValidator(add_validator) => {
                                    let Validator { pub_key, power } = add_validator;
                                    let pub_key = PublicKey {
                                        sum: Some(Sum::Ed25519(pub_key)),
                                    };
                                    let v = ValidatorUpdate {
                                        pub_key: Some(pub_key),
                                        power: power as i64,
                                    };
                                    validator_updates.push(v);
                                }
                                _ => {}
                            }
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
                store_apphash(&mut db_tx, &state.hash, &apphash).await?;

                db_tx.commit().await?;
                Ok::<_, ZCVError>((tx_results, validator_updates))
            })
            .expect("Fatal Failure in FinalizeBlock");

        ResponseFinalizeBlock {
            tx_results,
            validator_updates,
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
    let url = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();
    let rep = client
        .post(&url)
        .timeout(Duration::from_secs(300))
        .json(&req_body)
        .send()
        .await?
        .error_for_status()?;
    // broadcast_tx_sync returns the result of check_tx
    // .result.{code, log}
    // promote the log into an error message if code is not 0
    let mut json_rep: Value = rep.json().await?;
    if let Some(code) = json_rep.pointer("/result/code").and_then(|v| v.as_i64())
        && code != 0
    {
        let message = json_rep
            .pointer("/result/log")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        json_rep = json!({
            "id": "",
            "error": {
                "code": code,
                "message": message
            }
        });
    }

    Ok(json_rep)
}

pub async fn run_cometbft_app(
    context: Arc<tokio::sync::Mutex<Context>>,
    hash: &[u8],
    port: u16,
) -> ZCVResult<()> {
    let pool = {
        let c = context.lock().await;
        c.pool.clone()
    };
    let app = Server::new(pool, hash).await?;
    let server = ServerBuilder::new(1_000_000)
        .bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port), app)
        .anyhow()?;
    server.listen().anyhow()?;
    Ok(())
}
