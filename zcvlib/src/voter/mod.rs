use bigdecimal::{BigDecimal, ToPrimitive, num_bigint::BigInt};
use zcvlib::{api::Context, error::IntoAnyhow};

pub mod mutation;
pub mod query;
pub mod sync;

#[derive(Clone)]
pub struct GQLContext(pub Context);

impl juniper::Context for GQLContext {}

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

pub fn from_zats(v: u64) -> BigDecimal {
    let digits = BigInt::from(v);
    BigDecimal::from_bigint(digits, 8)
}
