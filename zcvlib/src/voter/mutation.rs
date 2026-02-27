use bigdecimal::BigDecimal;
use juniper::{FieldResult, GraphQLObject, graphql_object};
use zcvlib::db::set_account_seed;

use crate::voter::{GQLContext, from_zats, to_zats};

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
        zcvlib::api::simple::scan_notes(hash, id_account as u32, &(), &ctx.0).await?;
        Ok(true)
    }

    async fn scan_ballots(
        hash: String,
        id_accounts: Vec<i32>,
        context: &GQLContext,
    ) -> FieldResult<bool> {
        zcvlib::api::simple::scan_ballots(
            hash,
            id_accounts.into_iter().map(|a| a as u32).collect(),
            &context.0,
        )
        .await?;
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

    async fn collect_results(context: &GQLContext) -> FieldResult<Vec<VoteResultItem>> {
        let res = zcvlib::api::simple::collect_results(&context.0).await?;
        let res: Vec<_> = res
            .into_iter()
            .map(|v| VoteResultItem {
                idx_question: v.idx_question as i32,
                idx_sub_question: v.idx_sub_question as i32,
                idx_answer: v.idx_answer as i32,
                votes: from_zats(v.votes),
            })
            .collect();
        Ok(res)
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
}

#[derive(GraphQLObject)]
pub struct VoteResultItem {
    pub idx_question: i32,
    pub idx_sub_question: i32,
    pub idx_answer: i32,
    pub votes: BigDecimal,
}
