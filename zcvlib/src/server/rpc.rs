use std::sync::Arc;

use prost::Message;
use sqlx::query_as;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, async_trait};

use crate::{
    context::Context,
    error::IntoAnyhow,
    server::submit_tx,
    vote_rpc::{
        Ballot, Election, Empty, Hash, Validator, VoteHeight, VoteMessage, VoteRange,
        vote_message::TypeOneof, vote_streamer_server::VoteStreamer,
    },
};

pub struct ZCVServer {
    pub context: Arc<Mutex<Context>>,
}

#[async_trait]
impl VoteStreamer for ZCVServer {
    async fn set_election(&self, request: Request<Election>) -> Result<Response<Hash>, Status> {
        let res = async move {
            let election = request.into_inner();
            let m = VoteMessage {
                type_oneof: Some(TypeOneof::SetElection(election)),
            };
            let rep = self.submit(m).await?;
            let hash = rep
                .pointer("/result")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            Ok::<_, anyhow::Error>(Response::new(Hash {
                hash: hex::decode(hash)?,
            }))
        };
        res.await.map_err(to_tonic)
    }

    async fn add_validator(&self, request: Request<Validator>) -> Result<Response<Empty>, Status> {
        let res = async move {
            let validator = request.into_inner();
            let m = VoteMessage {
                type_oneof: Some(TypeOneof::AddValidator(validator)),
            };
            self.submit(m).await?;
            Ok::<_, anyhow::Error>(Response::new(Empty {}))
        };
        res.await.map_err(to_tonic)
    }

    async fn start(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        let m = VoteMessage {
            type_oneof: Some(TypeOneof::Start(0)),
        };
        self.submit(m).await?;
        Ok(Response::new(Empty {}))
    }

    async fn get_latest_vote_height(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<VoteHeight>, Status> {
        let res = async move {
            let c = self.context.lock().await;
            let mut conn = c.connect().await?;
            let (height, hash): (u32, Vec<u8>) =
                query_as("SELECT height, hash FROM state WHERE id = 0")
                    .fetch_one(&mut *conn)
                    .await?;
            Ok::<_, anyhow::Error>(Response::new(VoteHeight { height, hash }))
        };
        res.await.map_err(to_tonic)
    }

    type GetVoteRangeStream = ReceiverStream<Result<Ballot, Status>>;

    async fn get_vote_range(
        &self,
        request: Request<VoteRange>,
    ) -> Result<Response<Self::GetVoteRangeStream>, Status> {
        let res = async move {
            let request = request.into_inner();
            let VoteRange { start, end } = request;
            let conn = {
                let c = self.context.lock().await;
                c.pool.acquire().await?.detach()
            };
            let (tx, rx) = mpsc::channel::<Result<Ballot, Status>>(1);
            crate::db::get_ballot_range(conn, start, end, async move |b| {
                let _ = tx.send(Ok(b)).await;
                Ok(())
            })
            .await?;
            let rx = ReceiverStream::new(rx);
            let res = Response::new(rx);

            Ok::<_, anyhow::Error>(res)
        };
        res.await.map_err(to_tonic)
    }

    async fn submit_vote(&self, request: tonic::Request<Ballot>) -> Result<Response<Hash>, Status> {
        let res = async move {
            let ballot = request.into_inner();
            let m = VoteMessage {
                type_oneof: Some(TypeOneof::Ballot(ballot)),
            };
            let json = self.submit(m).await?;
            let hash = json.pointer("/result/data").and_then(|v| v.as_str()).unwrap_or_default();
            let hash = Hash { hash: hex::decode(hash)? };
            Ok(Response::new(hash))
        };
        res.await.map_err(to_tonic)
    }
}

impl ZCVServer {
    async fn submit(&self, m: VoteMessage) -> Result<serde_json::Value, Status> {
        let comet_port = {
            let c = self.context.lock().await;
            c.cometrpc_port
        };
        let res = submit_tx(m.encode_to_vec().as_slice(), comet_port)
            .await
            .anyhow()
            .map_err(to_tonic)?;
        if res.pointer("/error").is_some() {
            let error_message = res
                .pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            return Err(Status::internal(error_message));
        }
        Ok(res)
    }
}

pub fn to_tonic(e: anyhow::Error) -> tonic::Status {
    if let Some(status) = e.downcast_ref::<Status>() {
        return status.clone();
    }
    tonic::Status::internal(e.to_string())
}
