#[cfg(feature = "graphql")]
use bigdecimal::{BigDecimal, num_bigint::BigInt};
#[cfg(feature = "graphql")]
use juniper::{FieldError, FieldResult, Value, graphql_object};

use crate::voter::GQLContext;

#[cfg(feature = "graphql")]
pub struct Query {}

#[cfg(feature = "graphql")]
#[graphql_object]
#[graphql(context = GQLContext)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn compile_election_def(election_json: String, seed: String) -> FieldResult<String> {
        crate::api::simple::compile_election_def(election_json, seed)
        .map_err(|e| FieldError::new(e.to_string(), Value::Null))
    }

    async fn get_account_address(id_account: i32, context: &GQLContext) -> FieldResult<String> {
        let address = crate::api::simple::get_account_address(id_account as u32, &context.0).await?;
        Ok(address)
    }

    async fn get_balance(id_account: i32, context: &GQLContext) -> FieldResult<BigDecimal> {
        let b = crate::api::simple::get_balance(id_account as u32, &context.0).await?;
        let digits = BigInt::from(b);
        let zec = BigDecimal::from_bigint(digits, 8);
        Ok(zec)
    }
}
