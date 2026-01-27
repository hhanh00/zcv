use prost::Message;
use rocket::State;

use crate::{
    ZCVResult, context::Context, error::IntoAnyhow, server::submit_tx, vote_rpc::{Ballot, VoteMessage}
};

#[rocket::post("/submit_ballot", format = "json", data = "<ballot>")]
pub async fn submit_ballot(
    ballot: String,
    config: &State<Context>,
) -> ZCVResult<serde_json::Value> {
    let ballot_bytes = hex::decode(&ballot).anyhow()?;
    let b = Ballot {
        ballot: ballot_bytes,
    };
    let m = VoteMessage {
        type_oneof: Some(crate::vote_rpc::vote_message::TypeOneof::Ballot(b)),
    };
    submit_tx(m.encode_to_vec().as_slice(), config.comet_port).await
}
