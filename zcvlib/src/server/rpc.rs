use prost::Message;
use rocket::{State, serde::json::Json};

use crate::{
    ZCVResult,
    context::Context,
    server::submit_tx,
    vote_rpc::{Ballot, VoteMessage},
};

#[rocket::post("/submit_ballot", format = "json", data = "<ballot>")]
pub async fn submit_ballot(
    ballot: Json<orchard::vote::Ballot>,
    config: &State<Context>,
) -> ZCVResult<serde_json::Value> {
    let b = Ballot {
        ballot: serde_json::to_string(&ballot.into_inner()).unwrap(),
    };
    let m = VoteMessage {
        type_oneof: Some(crate::vote_rpc::vote_message::TypeOneof::Ballot(b)),
    };
    submit_tx(m.encode_to_vec().as_slice(), config.comet_port).await
}
