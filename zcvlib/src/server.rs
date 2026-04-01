use crate::{
    ZCVError, ZCVResult,
    context::BFTContext,
    db::{
        check_cmx_root, store_ballot,
        store_cmx_root, store_election, store_election_height_inc_position,
    },
    error::IntoAnyhow,
    pod::ElectionPropsPub,
    tiu,
    vote::VK,
    vote_rpc::{Ballot, Validator, VoteMessage, vote_message::TypeOneof},
};
use anyhow::anyhow;
use base64::{Engine, prelude::BASE64_STANDARD};
use blake2b_simd::Params;
use byteorder::{LE, ReadBytesExt};
use ff::PrimeField;
use incrementalmerkletree::{Hashable, Position, frontier::Frontier};
use orchard::tree::MerkleHashOrchard;
use pasta_curves::Fp;
use prost::{Message, bytes::Bytes};
use serde_json::{Value, json};
use sqlx::{
    Acquire, Sqlite, SqliteConnection, SqlitePool, Transaction, query,
};
use zcash_trees::warp::legacy::CommitmentTreeFrontier;
use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    io::{Cursor, Read},
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};
use tendermint_abci::{Application, ServerBuilder};
use tendermint_proto::{
    abci::{
        ExecTxResult, RequestCheckTx, RequestFinalizeBlock, RequestInfo, RequestPrepareProposal,
        RequestProcessProposal, ResponseCheckTx, ResponseCommit, ResponseFinalizeBlock,
        ResponseInfo, ResponsePrepareProposal, ResponseProcessProposal, ValidatorUpdate,
        response_process_proposal::ProposalStatus,
    },
    crypto::{PublicKey, public_key::Sum},
};
use tokio::{runtime::Runtime, sync::Mutex};
use zcash_encoding::Vector;

pub mod rpc;

pub type RPCResult<T> = Result<T, String>;

#[derive(Clone)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
}

impl Server {
    pub async fn new(
        pool: SqlitePool,
        lwd_url: &str,
        pir_url: &str,
        skip_validation: bool,
    ) -> ZCVResult<Self> {
        let server = ServerState::new(pool, lwd_url, pir_url, skip_validation).await?;
        Ok(Self {
            state: Arc::new(Mutex::new(server)),
        })
    }
}

pub struct ServerState {
    pub lwd_url: String,
    pub pir_url: String,
    pub pool: SqlitePool,
    pub locked: bool,
    pub election: Option<ElectionPropsPub>,
    pub skip_validation: bool,
    pub check_witnesses_cache: Arc<parking_lot::Mutex<HashMap<[u8; 32], bool>>>,
    pub domain: Fp,
    pub nf_root: MerkleHashOrchard,
    pub cmx_tree: Frontier<MerkleHashOrchard, 32>,

    pub db_tx: Option<Transaction<'static, Sqlite>>,
    pub apphash: [u8; 32],
}

impl ServerState {
    pub async fn new(
        pool: SqlitePool,
        lwd_url: &str,
        pir_url: &str,
        skip_validation: bool,
    ) -> ZCVResult<Self> {
        Ok(Self {
            pool,
            check_witnesses_cache: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            skip_validation,
            lwd_url: lwd_url.to_string(),
            pir_url: pir_url.to_string(),
            locked: false,
            election: None,
            domain: Fp::zero(),
            nf_root: MerkleHashOrchard::empty_leaf(),
            cmx_tree: Frontier::empty(),
            db_tx: None,
            apphash: [0u8; 32],
        })
    }
}

impl Application for Server {
    fn info(&self, _request: RequestInfo) -> ResponseInfo {
        ResponseInfo::default()
    }

    // Checks if a tx is structurally correct
    // Valid txs must not be rejected
    // But bad txs may be kept for the moment
    fn check_tx(&self, request: RequestCheckTx) -> ResponseCheckTx {
        tracing::info!("check_tx");
        let RequestCheckTx { mut tx, .. } = request;
        let rt = Runtime::new().unwrap();
        let data = rt.block_on(async move {
            let pool = {
                let state = self.state.lock().await;
                state.pool.clone()
            };
            let mut conn = pool.acquire().await?;
            let msg = VoteMessage::decode(&mut tx)?;
            let msg = msg.type_oneof.ok_or(anyhow!("Must have payload"))?;
            let res = match msg {
                TypeOneof::SetElection(election) => {
                    let state = self.state.lock().await;
                    if state.locked {
                        anyhow::bail!("Election cannot be added once the blockchain is locked.");
                    }
                    let election: ElectionPropsPub = serde_json::from_str(&election.election)?;
                    election.domain.clone()
                }
                TypeOneof::Ballot(ballot) => {
                    tracing::info!("check_tx::ballot");
                    let ballot = from_protobuf(&ballot)?;
                    let hash = ballot.data.sighash()?;
                    // Fail on inter block double spend (but pass on intra block
                    // duplicate because it checks against the db)
                    let (election, cache, domain, e_nf_root, skip_validation) = {
                        let state = self.state.lock().await;
                        let election = state.election.clone().ok_or(anyhow!("Election not set"))?;
                        let cache = state.check_witnesses_cache.clone();
                        (
                            election,
                            cache,
                            state.domain,
                            state.nf_root,
                            state.skip_validation,
                        )
                    };
                    ServerState::check_ballot(
                        &mut conn,
                        &election,
                        ballot,
                        domain,
                        e_nf_root,
                        cache,
                        skip_validation,
                    )
                    .await?;
                    tracing::info!("Ballot checked");
                    hash
                }
                TypeOneof::AddValidator(v) => {
                    let state = self.state.lock().await;
                    if state.locked {
                        anyhow::bail!("Validators cannot be added once the blockchain is locked.");
                    }
                    v.pub_key
                }
                TypeOneof::Lock(_) => {
                    let state = self.state.lock().await;
                    if state.locked {
                        anyhow::bail!("Blockchain already locked.");
                    }
                    Vec::new()
                }
            };
            Ok::<_, anyhow::Error>(res)
        });

        match data {
            Ok(data) => ResponseCheckTx {
                code: 0,
                data: data.into(),
                ..Default::default()
            },
            Err(err) => {
                tracing::info!("check_tx error: {}", err.to_string());
                ResponseCheckTx {
                    code: 1,
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
            txs, max_tx_bytes, ..
        } = request;
        let max_tx_bytes = max_tx_bytes as usize;

        let rt = Runtime::new().unwrap();
        let proposed_txs = rt
            .block_on(async move {
                let mut nfs: HashSet<[u8; 32]> = HashSet::new();
                let mut proposed_txs = vec![];
                let mut proposed_len = 0;
                'next_tx: for tx in txs {
                    let mut tx2 = tx.clone();
                    let msg = VoteMessage::decode(&mut tx2)?;
                    // expect was checked by check_tx
                    let m = msg.type_oneof.expect("VoteMessage must have content");
                    if let TypeOneof::Ballot(ballot) = m {
                        let ballot = from_protobuf(&ballot).anyhow()?;
                        tracing::info!(
                            "Proposing ballot {}...",
                            hex::encode(ballot.data.sighash()?)
                        );
                        for a in ballot.data.actions {
                            let nf: [u8; 32] = tiu!(a.nf);
                            // Do not include double spend mempool tx
                            if nfs.contains(&nf) {
                                tracing::info!("Duplicate tx intra block");
                                continue 'next_tx;
                            }
                            nfs.insert(nf);
                        }
                    }

                    if proposed_len + tx.len() > max_tx_bytes {
                        break;
                    }
                    proposed_len += tx.len();
                    proposed_txs.push(tx);
                    tracing::info!("... Added");
                }
                Ok::<_, anyhow::Error>(proposed_txs)
            })
            .expect("Error in proposed_tx");

        ResponsePrepareProposal { txs: proposed_txs }
    }

    // A proposal can come from another node
    // We should not trust it and validate the txs ourself
    fn process_proposal(&self, request: RequestProcessProposal) -> ResponseProcessProposal {
        let RequestProcessProposal { txs, height, .. } = request;
        // Reject ill formed proposals
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(async move {
            let state = self.state.lock().await;
            let mut conn = state.pool.acquire().await?;
            let election = state.election.clone();
            // Check everything (do the same thing as finalize_block) but do not commit
            let mut db_tx = conn.begin().await?;
            for (itx, mut tx) in txs.into_iter().enumerate() {
                let msg = VoteMessage::decode(&mut tx)?;
                if let Some(TypeOneof::Ballot(ballot)) = msg.type_oneof {
                    let ballot = from_protobuf(&ballot).anyhow()?;
                    if let Some(election) = &election {
                        ServerState::check_witnesses(
                            &mut db_tx,
                            election,
                            &ballot,
                            state.domain,
                            state.nf_root,
                            state.check_witnesses_cache.clone(),
                            state.skip_validation,
                        )
                        .await?;
                        store_ballot(&mut db_tx, height as u32, itx as u32, ballot).await?;
                    } else {
                        anyhow::bail!("No election set");
                    }
                }
            }
            db_tx.rollback().await?;
            Ok::<_, anyhow::Error>(())
        });
        let status = match res {
            Ok(_) => ResponseProcessProposal {
                status: ProposalStatus::Accept as i32,
            },
            Err(_) => ResponseProcessProposal {
                status: ProposalStatus::Reject as i32,
            },
        };
        tracing::info!("{status:?}");
        status
    }

    // Process the block that was voted on by the validators
    fn finalize_block(&self, request: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        let RequestFinalizeBlock {
            txs, hash, height, ..
        } = request;
        tracing::info!(
            "Hash {} height {height} {} txs",
            hex::encode(&hash),
            txs.len()
        );
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (app_hash, tx_results, validator_updates) = rt
            .block_on(async move {
                let mut state = self.state.lock().await;
                let mut validator_updates = vec![];
                let mut db_tx = state.pool.begin().await?;
                let apphash = &state.apphash;
                let mut tx_results = vec![];
                let new_apphash = if txs.is_empty() {
                    *apphash
                } else {
                    let mut hasher = Params::new()
                        .personal(b"ZCVote___AppHash")
                        .hash_length(32)
                        .key(apphash.as_slice())
                        .to_state();
                    for (itx, mut tx) in txs.into_iter().enumerate() {
                        let tx_copy = tx.clone();
                        hasher.update(&tx_copy);
                        let finalize = async {
                            let msg = VoteMessage::decode(&mut tx)?;
                            // expect was checked by check_tx
                            let m = msg.type_oneof.expect("VoteMessage must have content");
                            match m {
                                TypeOneof::SetElection(election) => {
                                    let crate::vote_rpc::Election {
                                        election,
                                        nf_root,
                                        cmx_tree_state,
                                    } = election;
                                    let election: ElectionPropsPub =
                                        serde_json::from_str(&election)?;
                                    store_election(&mut db_tx, &election).await?;

                                    tracing::info!("NF ROOT: {}", hex::encode(&nf_root));
                                    // store_roots(&mut db_tx, &nf_root, &cmx_tree_state).await?;

                                    let (nf_root, cmx_tree) =
                                        read_roots(&nf_root, &cmx_tree_state)?;

                                    let cmx_root = cmx_tree.root();
                                    tracing::info!(
                                        "CMX ROOT: {}",
                                        hex::encode(cmx_root.to_bytes())
                                    );

                                    store_cmx_root(&mut db_tx, &cmx_root.to_bytes(), election.end)
                                        .await?;
                                    let domain =
                                        Fp::from_repr(tiu!(election.domain.clone())).unwrap();

                                    state.election = Some(election);
                                    state.domain = domain;
                                    state.nf_root = nf_root;
                                    state.cmx_tree = cmx_tree;
                                }
                                TypeOneof::Ballot(ballot) => {
                                    tracing::info!("Incoming ballot");
                                    let ballot = from_protobuf(&ballot).anyhow()?;
                                    let hash = ballot.data.sighash()?;
                                    let election = state
                                        .election
                                        .clone()
                                        .ok_or(anyhow!("Election not set"))?;
                                    let h = election.end + height as u32;
                                    tracing::info!(
                                        "Expected NF ROOT: {}",
                                        hex::encode(state.nf_root.to_bytes())
                                    );
                                    tracing::info!(
                                        "Expected CMX ROOT: {}",
                                        hex::encode(state.cmx_tree.root().to_bytes())
                                    );
                                    ServerState::check_witnesses(
                                        &mut db_tx,
                                        &election,
                                        &ballot,
                                        state.domain,
                                        state.nf_root,
                                        state.check_witnesses_cache.clone(),
                                        state.skip_validation,
                                    )
                                    .await?;
                                    for a in ballot.data.actions.iter() {
                                        let cmx = MerkleHashOrchard::from_bytes(&a.cmx).unwrap();
                                        state.cmx_tree.append(cmx);
                                    }
                                    // This will catch and fail on a double spend because of the UNIQUE dnf
                                    let id_ballot =
                                        store_ballot(&mut db_tx, h, itx as u32, ballot).await?;
                                    if id_ballot.is_none() {
                                        tracing::info!(
                                            "Tx already inserted {}",
                                            hex::encode(&hash)
                                        );
                                    }
                                    store_election_height_inc_position(&mut db_tx, h).await?;
                                }
                                TypeOneof::Lock(_) => {
                                    state.locked = true;
                                    query("UPDATE v_state SET locked = 1 WHERE id = 0")
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
                    tiu!(hasher.finalize().as_bytes())
                };
                let height = height as u32;
                store_cmx_root(&mut db_tx, &state.cmx_tree.root().to_bytes(), height).await?;

                state.clear_check_witnesses();
                state.db_tx = Some(db_tx);
                Ok::<_, ZCVError>((new_apphash, tx_results, validator_updates))
            })
            .expect("Fatal Failure in FinalizeBlock");

        ResponseFinalizeBlock {
            tx_results,
            validator_updates,
            app_hash: Bytes::from(app_hash.to_vec()),
            ..ResponseFinalizeBlock::default()
        }
    }

    fn commit(&self) -> ResponseCommit {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut state = self.state.lock().await;
            if let Some(db_tx) = state.db_tx.take() {
                db_tx.commit().await?;
            }
            Ok::<_, ZCVError>(())
        })
        .expect("DB Commit failed");
        ResponseCommit::default()
    }
}

pub async fn check_dup_nf(conn: &mut SqliteConnection, nf: &[u8]) -> ZCVResult<bool> {
    let exists = query("SELECT 1 FROM v_actions WHERE dnf = ?1")
        .bind(nf)
        .fetch_optional(&mut *conn)
        .await?
        .is_some();
    Ok(exists)
}

fn read_roots(
    nf_root: &[u8],
    cmx_tree: &[u8],
) -> ZCVResult<(
    MerkleHashOrchard,
    incrementalmerkletree::frontier::Frontier<MerkleHashOrchard, 32>,
)> {
    let nf_root = MerkleHashOrchard::from_bytes(&tiu!(nf_root)).unwrap();
    let cmx_tree = CommitmentTreeFrontier::read(&*cmx_tree).anyhow()?;
    let cmx_tree = cmx_tree.to_orchard_frontier();
    Ok((nf_root, cmx_tree))
}

impl ServerState {
    pub fn clear_check_witnesses(&mut self) {
        let mut cache = self.check_witnesses_cache.lock();
        cache.clear();
    }

    pub async fn check_witnesses(
        conn: &mut SqliteConnection,
        e: &ElectionPropsPub,
        ballot: &orchard::vote::Ballot,
        e_domain: Fp,
        e_nf_root: MerkleHashOrchard,
        cache: Arc<parking_lot::Mutex<HashMap<[u8; 32], bool>>>,
        skip_validation: bool,
    ) -> ZCVResult<()> {
        let sighash: [u8; 32] = tiu!(ballot.data.sighash().unwrap());
        {
            let mut check_witnesses_cache = cache.lock();
            let cached = check_witnesses_cache.entry(sighash);
            if let Entry::Occupied(valid) = cached {
                if !valid.get() {
                    return Err(ZCVError::Any(anyhow!(
                        "Ballot previously checked as invalid"
                    )));
                }
                tracing::info!("Witness checked (cached)");
                return Ok(());
            }
        }

        if !skip_validation {
            let domain = Fp::from_repr(ballot.data.domain)
                .into_option()
                .ok_or(anyhow!("Invalid domain"))?;
            if e_domain != domain {
                return Err(ZCVError::Any(anyhow!("Ballot has unexpected domain")));
            }
            let nf_root = MerkleHashOrchard::from_bytes(&ballot.data.anchors.nf)
                .into_option()
                .ok_or(anyhow!("Ballot has invalid nf root"))?;
            if e_nf_root != nf_root {
                return Err(ZCVError::Any(anyhow!("Ballot has unexpected nf root")));
            }
            let cmx_root = MerkleHashOrchard::from_bytes(&ballot.data.anchors.cmx)
                .into_option()
                .ok_or(anyhow!("Ballot has invalid cmx root"))?;
            check_cmx_root(conn, &cmx_root.to_bytes()).await?;

            tracing::info!("Public anchors checked");
            orchard::vote::validate_ballot(ballot.clone(), e.need_sig, &VK)?;
            tracing::info!("Witness checked");
        }
        {
            let mut check_witnesses_cache = cache.lock();
            check_witnesses_cache.insert(sighash, true);
        }
        Ok(())
    }

    pub async fn check_ballot(
        conn: &mut SqliteConnection,
        election: &ElectionPropsPub,
        ballot: orchard::vote::Ballot,
        e_domain: Fp,
        e_nf_root: MerkleHashOrchard,
        cache: Arc<parking_lot::Mutex<HashMap<[u8; 32], bool>>>,
        skip_validation: bool,
    ) -> ZCVResult<()> {
        Self::check_witnesses(
            conn,
            election,
            &ballot,
            e_domain,
            e_nf_root,
            cache,
            skip_validation,
        )
        .await?;
        for a in ballot.data.actions {
            tracing::info!("Action NF: {}", hex::encode(a.nf));
            let exists = check_dup_nf(conn, a.nf.as_slice()).await?;
            if exists {
                tracing::info!("Duplicate tx inter block (skip from proposal)");
                return Err(ZCVError::Duplicate);
            }
        }
        Ok(())
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
    let ballot = orchard::vote::Ballot::read(&*ballot.ballot)?;
    Ok(ballot)
}

pub async fn run_cometbft_app(
    context: Arc<tokio::sync::Mutex<BFTContext>>,
    port: u16,
) -> ZCVResult<()> {
    let (pool, lwd_url, pir_url, skip_validation) = {
        let c = context.lock().await;
        (
            c.context.pool.clone(),
            c.context.lwd_url.clone(),
            c.context.pir_url.clone(),
            c.skip_validation,
        )
    };
    let app = Server::new(pool, &lwd_url, &pir_url, skip_validation).await?;
    let server = ServerBuilder::new(1_000_000)
        .bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port), app)
        .anyhow()?;
    server.listen().anyhow()?;
    Ok(())
}
