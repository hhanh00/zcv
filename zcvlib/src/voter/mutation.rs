use bigdecimal::{BigDecimal, ToPrimitive};
use juniper::{FieldResult, graphql_object};
use zcvlib::{db::set_account_seed, error::IntoAnyhow};

use crate::voter::GQLContext;

pub struct Mutation {}

#[graphql_object]
#[graphql(
    context = GQLContext,
)]
impl Mutation {
    async fn set_seed(
        seed: String,
        id_account: i32,
        aindex: i32,
        ctx: &GQLContext,
    ) -> FieldResult<bool> {
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

    async fn scan_ballots(
        hash: String,
        id_accounts: Vec<i32>,
        context: &GQLContext,
    ) -> FieldResult<bool> {
        zcvlib::api::simple::scan_ballots(hash, id_accounts.into_iter().map(|a| a as u32).collect(), &context.0).await?;
        Ok(true)
    }

    async fn decode_ballots(
        hash: String,
        election_seed: String,
        context: &GQLContext,
    ) -> FieldResult<bool> {
        zcvlib::api::simple::decode_ballots(hash, election_seed, &context.0).await?;
        Ok(true)
    }

    async fn collect_results(context: &GQLContext) -> FieldResult<bool> {
        zcvlib::api::simple::collect_results(&context.0).await?;
        Ok(true)
    }

    async fn vote(
        hash: String,
        id_account: i32,
        idx_question: i32,
        vote_content: String,
        amount: BigDecimal,
        ctx: &GQLContext,
    ) -> FieldResult<bool> {
        let amount = to_zats(amount)?;
        zcvlib::api::simple::vote(
            hash,
            id_account as u32,
            idx_question as u32,
            vote_content,
            amount,
            &ctx.0,
        )
        .await?;
        Ok(true)
    }

    async fn mint(
        hash: String,
        id_account: i32,
        idx_question: i32,
        amount: BigDecimal,
        ctx: &GQLContext,
    ) -> FieldResult<bool> {
        let amount = to_zats(amount)?;
        zcvlib::api::simple::mint(hash, id_account as u32, idx_question as u32, amount, &ctx.0)
            .await?;
        Ok(true)
    }

    async fn delegate(
        hash: String,
        id_account: i32,
        idx_question: i32,
        address: String,
        amount: BigDecimal,
        ctx: &GQLContext,
    ) -> FieldResult<bool> {
        let amount = to_zats(amount)?;
        tracing::info!("delegate {amount}");
        zcvlib::api::simple::delegate(
            hash,
            id_account as u32,
            idx_question as u32,
            &address,
            amount,
            &ctx.0,
        )
        .await?;
        Ok(true)
    }

    // pub async fn tally_election(
    //     hash: String,
    //     election_seed: String,
    //     ctx: &GQLContext,
    // ) -> FieldResult<bool> {
    //     zcvlib::api::simple::tally_election(hash, election_seed, &ctx.0).await?;
    //     Ok(true)
    // }
}

fn to_zats(v: BigDecimal) -> anyhow::Result<u64> {
    let zats = v
        .with_scale(8)
        .as_bigint_and_exponent()
        .0
        .to_u64()
        .ok_or(anyhow::anyhow!("Invalid amount"))
        .anyhow()?;
    Ok(zats)
}
