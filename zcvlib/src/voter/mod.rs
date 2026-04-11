pub mod mutation;
pub mod query;
pub mod sync;

#[cfg(feature = "graphql")]
use bigdecimal::{BigDecimal, ToPrimitive, num_bigint::BigInt};

#[cfg(feature = "graphql")]
use crate::context::Context;

#[cfg(feature = "graphql")]
#[derive(Clone)]
pub struct GQLContext(pub Context);

#[cfg(feature = "graphql")]
impl juniper::Context for GQLContext {}

#[cfg(feature = "graphql")]
fn to_zats(v: BigDecimal) -> anyhow::Result<u64> {
    let zats = v
        .with_scale(8)
        .as_bigint_and_exponent()
        .0
        .to_u64()
        .ok_or(anyhow::anyhow!("Invalid amount"))?;
    Ok(zats)
}

#[cfg(feature = "graphql")]
pub fn from_zats(v: u64) -> BigDecimal {
    let digits = BigInt::from(v);
    BigDecimal::from_bigint(digits, 8)
}
