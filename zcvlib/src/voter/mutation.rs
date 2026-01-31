use juniper::{FieldResult, graphql_object};
use zcvlib::db::set_account_seed;

use crate::voter::Context;

pub struct Mutation {}

#[graphql_object]
#[graphql(
    context = Context,
)]
impl Mutation {
    async fn set_seed(
        seed: String,
        context: &Context,
    ) -> FieldResult<bool> {
        let mut conn = context.connect().await?;
        set_account_seed(&mut conn, 1, &seed, 0).await?;
        Ok(true)
    }
}
