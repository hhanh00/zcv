use bigdecimal::{BigDecimal, num_bigint::BigInt};
use juniper::{FieldError, FieldResult, Value, graphql_object};

use crate::voter::GQLContext;

pub struct Query {}

#[graphql_object]
#[graphql(context = GQLContext)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn compile_election_def(election_json: String, seed: String) -> FieldResult<String> {
        zcvlib::api::simple::compile_election_def(election_json, seed)
        .map_err(|e| FieldError::new(e.to_string(), Value::Null))
    }

    async fn get_account_address(id_account: i32, context: &GQLContext) -> FieldResult<String> {
        let address = zcvlib::api::simple::get_account_address(id_account as u32, &context.0).await?;
        Ok(address)
    }

    async fn get_balance(hash: String, id_account: i32, idx_question: i32, context: &GQLContext) -> FieldResult<BigDecimal> {
        let b = zcvlib::api::simple::get_balance(hash, id_account as u32, idx_question as u32, &context.0).await?;
        let digits = BigInt::from(b);
        let zec = BigDecimal::from_bigint(digits, 8);
        Ok(zec)
    }
}
