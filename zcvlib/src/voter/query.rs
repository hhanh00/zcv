use juniper::{FieldResult, graphql_object};
use zcvlib::pod::ElectionProps;

use crate::voter::Context;

pub struct Query {}

#[graphql_object]
#[graphql(context = Context)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn compile_election_def(election_yaml: String, seed: String) -> FieldResult<String> {
        let election: ElectionProps = serde_json::from_str(&election_yaml)?;
        let epub = election.build(&seed)?;
        let res = serde_json::to_string(&epub).unwrap();
        Ok(res)
    }
}
