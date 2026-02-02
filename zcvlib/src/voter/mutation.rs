use juniper::{FieldResult, graphql_object};
use zcvlib::db::set_account_seed;

use crate::voter::GQLContext;

pub struct Mutation {}

#[graphql_object]
#[graphql(
    context = GQLContext,
)]
impl Mutation {
    const ID_ACCOUNT: u32 = 1;

    async fn set_seed(seed: String, ctx: &GQLContext) -> FieldResult<bool> {
        let mut conn = ctx.0.connect().await?;
        set_account_seed(&mut conn, Self::ID_ACCOUNT, &seed, 0).await?;
        Ok(true)
    }

    async fn store_election(election_json: String, ctx: &GQLContext) -> FieldResult<String> {
        let hash = zcvlib::api::simple::store_election(election_json, &ctx.0).await?;
        Ok(hex::encode(&hash))
    }

    async fn scan_notes(hash: String, ctx: &GQLContext) -> FieldResult<bool> {
        zcvlib::api::simple::scan_notes(hash, Self::ID_ACCOUNT, &ctx.0).await?;
        Ok(true)
    }
}
