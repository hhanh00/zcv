use std::sync::Arc;

use crate::voter::{Context, mutation::Mutation, query::Query};
use anyhow::Result;
use clap::Parser;
use juniper::{EmptySubscription, RootNode};
use serde::{Deserialize, Serialize};
use warp::Filter;

pub mod voter;

type Schema = RootNode<Query, Mutation, EmptySubscription<Context>>;

#[derive(Parser, Serialize, Deserialize, Debug)]
pub struct Config {
    #[clap(short, long, value_parser)]
    pub db_path: Option<String>,
    #[clap(short, long, value_parser)]
    pub lwd_url: Option<String>,
    #[clap(short, long, value_parser)]
    pub port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .compact()
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let config = Config::parse();
    let Config {
        db_path,
        lwd_url,
        port,
    } = config;
    let db_path = db_path.unwrap_or("voter.db".to_string());
    let lwd_url = lwd_url.unwrap_or("https://zec.rocks".to_string());
    let port = port.unwrap_or(8000);

    let context = Context::new(&db_path, &lwd_url).await?;

    let schema = Schema::new(Query {}, Mutation {}, EmptySubscription::default());

    let context_extractor = warp::any().map(move || context.clone());

    let schema = Arc::new(schema);

    let routes = (warp::post()
        .and(warp::path("graphql"))
        .and(juniper_warp::make_graphql_filter(
            schema.clone(),
            context_extractor,
        )))
    .or(warp::get()
        .and(warp::path("graphiql"))
        .and(juniper_warp::graphiql_filter("/graphql", None)));

    tracing::info!("Listening on 127.0.0.1:{port}");
    warp::serve(routes).run(([127, 0, 0, 1], port)).await;

    Ok(())
}
