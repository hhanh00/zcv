use crate::{
    ZCVError, ZCVResult,
    context::BFTContext,
    db::{get_apphash, store_apphash, store_ballot, store_election},
    error::IntoAnyhow,
    pod::ElectionPropsPub,
    vote_rpc::{Ballot, Validator, VoteMessage, vote_message::TypeOneof},
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
    pub started: bool,
    pub election: Option<ElectionPropsPub>,
}

impl ServerState {
    pub async fn new(pool: SqlitePool, hash: &[u8]) -> ZCVResult<Self> {
        let mut conn = pool.acquire().await?;
        let (started,): (bool,) = query_as("SELECT started FROM state WHERE id = 0")
            .fetch_one(&mut *conn)
            .await?;

        Ok(Self {
            pool,
            hash: hash.to_vec(),
            started,
            election: None,
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
            let msg = msg.type_oneof.ok_or(anyhow!("Must have payload"))?;
            let res = match msg {
                TypeOneof::SetElection(election) => {
                    let election: ElectionPropsPub = serde_json::from_str(&election.election)?;
                    election.hash()?.to_vec()
                }
                TypeOneof::Ballot(ballot) => {
                    let mut state = self.state.lock();
                    let ballot = from_protobuf(&ballot)?;
                    let hash = ballot.data.sighash()?;
                    state.check_ballot(ballot)?;
                    hash
                }
                TypeOneof::AddValidator(v) => {
                    let state = self.state.lock();
                    if state.started {
                        anyhow::bail!("Validators cannot be added once the blockchain is started.");
                    }
                    v.pub_key
                }
                TypeOneof::Start(_) => {
                    let state = self.state.lock();
                    if state.started {
                        anyhow::bail!("Blockchain already started.");
                    }
                    Vec::new()
                }
            };
            Ok::<_, anyhow::Error>(res)
        };

        match check() {
            Ok(data) => ResponseCheckTx {
                code: 0,
                data: data.into(),
                ..Default::default()
            },
            Err(err) => {
                tracing::info!("check_tx error: {}", err.to_string());
                ResponseCheckTx {
                    code: 1,
                    data: tx,
                    log: err.to_string(),
                    ..Default::default()
                }
            }
        }
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
                let apphash = get_apphash(&mut db_tx, &state.hash)
                    .await?
                    .unwrap_or_default();
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
                        // expect was checked by check_tx
                        let m = msg.type_oneof.expect("VoteMessage must have content");
                        match m {
                            TypeOneof::SetElection(election) => {
                                let election: ElectionPropsPub =
                                    serde_json::from_str(&election.election)?;
                                store_election(&mut db_tx, &election).await?;
                                state.election = Some(election);
                            }
                            TypeOneof::Ballot(ballot) => {
                                tracing::info!("Incoming ballot");
                                let ballot = from_protobuf(&ballot).anyhow()?;
                                let hash = ballot.data.sighash()?;
                                let election =
                                    state.election.as_ref().ok_or(anyhow!("Election not set"))?;
                                let rows_added = store_ballot(
                                    &mut db_tx,
                                    election.start + height as u32,
                                    itx as u32,
                                    ballot,
                                )
                                .await?;
                                tracing::info!("{rows_added} tx added to db");
                                if rows_added != 1 {
                                    tracing::info!("Tx already inserted {}", hex::encode(&hash));
                                }
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
                        }
                        Ok::<_, anyhow::Error>(())
                    };
                    let result = match finalize.await {
                        Ok(_) => ExecTxResult::default(),
                        Err(error) => {
                            tracing::info!("Finalization error: {}", error);
                            ExecTxResult {
                                code: 1,
                                data: tx_copy,
                                log: error.to_string(),
                                info: "Error in finalization".to_string(),
                                ..ExecTxResult::default()
                            }
                        }
                    };
                    tx_results.push(result);
                }
                let apphash = hasher.finalize().as_bytes().to_vec();
                store_apphash(&mut db_tx, &state.hash, &apphash)
                    .await
                    .unwrap();

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
    tracing::info!("submit_tx: {:?}", json_rep);
    if let Some(code) = json_rep.pointer("/result/code").and_then(|v| v.as_i64())
        && code != 0
    {
        let message = json_rep.pointer("/result/log").and_then(|v| v.as_str());
        let message = message.unwrap_or_default().to_string();
        json_rep = json!({
            "id": "",
            "error": {
                "code": code,
                "message": message
            }
        });
    } else if let Some(code) = json_rep.pointer("/error/code").and_then(|v| v.as_i64()) {
        let message = json_rep.pointer("/error/data").and_then(|v| v.as_str());
        let message = message.unwrap_or_default().to_string();
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

pub fn from_protobuf(ballot: &Ballot) -> std::io::Result<orchard::vote::Ballot> {
    let data = orchard::vote::BallotData::read(&*ballot.data)?;
    let witnesses = orchard::vote::BallotWitnesses::read(&*ballot.witnesses)?;
    Ok(orchard::vote::Ballot { data, witnesses })
}

pub async fn run_cometbft_app(
    context: Arc<tokio::sync::Mutex<BFTContext>>,
    hash: &[u8],
    port: u16,
) -> ZCVResult<()> {
    let pool = {
        let c = context.lock().await;
        c.context.pool.clone()
    };
    let app = Server::new(pool, hash).await?;
    let server = ServerBuilder::new(1_000_000)
        .bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port), app)
        .anyhow()?;
    server.listen().anyhow()?;
    Ok(())
}
