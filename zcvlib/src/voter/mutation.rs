use juniper::{FieldResult, graphql_object};
use zcvlib::db::set_account_seed;

use crate::voter::GQLContext;

pub struct Mutation {}

#[graphql_object]
#[graphql(
    context = GQLContext,
)]
impl Mutation {
    async fn set_seed(seed: String, ctx: &GQLContext) -> FieldResult<bool> {
        let mut conn = ctx.0.connect().await?;
        set_account_seed(&mut conn, 1, &seed, 0).await?;
        Ok(true)
    }

    async fn store_election(election_json: String, ctx: &GQLContext) -> FieldResult<i32> {
        let id = zcvlib::api::simple::store_election(election_json, &ctx.0).await?;
        Ok(id as i32)
    }
}
