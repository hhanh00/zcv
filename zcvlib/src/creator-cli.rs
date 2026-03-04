use std::fs::File;

use anyhow::Result;
use clap::Parser;
use figment::{
    Figment,
    providers::{Format, Yaml},
};
use serde::{Deserialize, Serialize};
use zcvlib::pod::ElectionProps;

#[derive(Parser, Serialize, Deserialize, Debug)]
pub struct Config {
    #[clap(short, long, value_parser)]
    pub election_file: String,
    #[clap(short, long, value_parser)]
    pub seed: String,
    #[clap(short, long, value_parser)]
    pub output_file: String,
}

fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .compact()
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let config = Config::parse();
    create_election_file(&config)?;
    Ok(())
}

pub fn create_election_file(config: &Config) -> Result<Vec<u8>> {
    let election: ElectionProps = Figment::new()
        .merge(Yaml::file(&config.election_file))
        .extract()?;
    let e = election.build(&config.seed)?;
    let mut output = File::create(&config.output_file)?;
    serde_json::to_writer_pretty(&mut output, &e)?;
    tracing::info!("Election domain: {}", hex::encode(&e.domain));
    Ok(e.domain.clone())
}
