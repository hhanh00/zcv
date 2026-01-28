use std::sync::Arc;

use prost::Message;
use sqlx::query_as;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, async_trait};

use crate::{
    ZCVResult,
    context::Context,
    error::IntoAnyhow,
    server::submit_tx,
    vote_rpc::{
        Ballot, Empty, Hash, VoteHeight, VoteMessage, VoteRange, vote_streamer_server::VoteStreamer,
    },
};

pub struct ZCVServer {
    pub context: Arc<Mutex<Context>>,
}

#[async_trait]
impl VoteStreamer for ZCVServer {
    async fn get_latest_vote_height(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<VoteHeight>, Status> {
        let res = async move {
            println!("HAHA");
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
                type_oneof: Some(crate::vote_rpc::vote_message::TypeOneof::Ballot(ballot)),
            };
            let comet_port = {
                let c = self.context.lock().await;
                c.cometrpc_port
            };
            let json = submit_tx(m.encode_to_vec().as_slice(), comet_port).await?;
            tracing::info!("{json:?}");
            let hash = Hash { hash: vec![] };
            Ok(Response::new(hash))
        };
        res.await.map_err(to_tonic)
    }
}

pub fn to_tonic(e: anyhow::Error) -> tonic::Status {
    tonic::Status::internal(e.to_string())
}
