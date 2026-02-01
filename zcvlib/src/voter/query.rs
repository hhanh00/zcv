use juniper::{FieldError, FieldResult, Value, graphql_object};

use crate::voter::Context;

pub struct Query {}

#[graphql_object]
#[graphql(context = Context)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn compile_election_def(election_json: String, seed: String) -> FieldResult<String> {
        zcvlib::api::simple::compile_election_def(election_json, seed)
        .map_err(|e| FieldError::new(e.to_string(), Value::Null))
    }
}
