use bigdecimal::{BigDecimal, ToPrimitive};
use juniper::{FieldResult, graphql_object};
use zcvlib::db::set_account_seed;

use crate::voter::GQLContext;

pub struct Mutation {}

#[graphql_object]
#[graphql(
    context = GQLContext,
)]
impl Mutation {
    async fn set_seed(seed: String, id_account: i32, aindex: i32, ctx: &GQLContext) -> FieldResult<bool> {
        let mut conn = ctx.0.connect().await?;
        set_account_seed(&mut conn, id_account as u32, &seed, aindex as u32).await?;
        Ok(true)
    }

    async fn store_election(election_json: String, ctx: &GQLContext) -> FieldResult<String> {
        let hash = zcvlib::api::simple::store_election(election_json, &ctx.0).await?;
        Ok(hex::encode(&hash))
    }

    async fn scan_notes(hash: String, id_account: i32, ctx: &GQLContext) -> FieldResult<bool> {
        zcvlib::api::simple::scan_notes(hash, id_account as u32, &ctx.0).await?;
        Ok(true)
    }

    async fn vote(hash: String, id_account: i32, idx_question: i32, vote_content: String, amount: BigDecimal, ctx: &GQLContext) -> FieldResult<bool> {
        let amount = amount.with_scale(8).to_u64().ok_or(anyhow::anyhow!("Invalid amount"))?;
        zcvlib::api::simple::vote(hash, id_account as u32, idx_question as u32, vote_content, amount, &ctx.0).await?;
        Ok(true)
    }

    async fn mint(hash: String, id_account: i32, idx_question: i32, amount: BigDecimal, ctx: &GQLContext) -> FieldResult<bool> {
        let amount = amount.with_scale(8).to_u64().ok_or(anyhow::anyhow!("Invalid amount"))?;
        zcvlib::api::simple::mint(hash, id_account as u32, idx_question as u32, amount, &ctx.0).await?;
        Ok(true)
    }

    async fn delegate(hash: String, id_account: i32, idx_question: i32, address: String, amount: BigDecimal, ctx: &GQLContext) -> FieldResult<bool> {
        let amount = amount.with_scale(8).to_u64().ok_or(anyhow::anyhow!("Invalid amount"))?;
        zcvlib::api::simple::delegate(hash, id_account as u32, idx_question as u32, &address, amount, &ctx.0).await?;
        Ok(true)
    }
}
