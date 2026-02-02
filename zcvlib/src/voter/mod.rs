use zcvlib::api::Context;

pub mod sync;
pub mod query;
pub mod mutation;

#[derive(Clone)]
pub struct GQLContext(pub Context);

impl juniper::Context for GQLContext {}

