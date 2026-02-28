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

pub fn create_election_file(config: &Config) -> Result<[u8; 32]> {
    let election: ElectionProps = Figment::new()
        .merge(Yaml::file(&config.election_file))
        .extract()?;
    let e = election.build(&config.seed)?;
    let mut output = File::create(&config.output_file)?;
    serde_json::to_writer_pretty(&mut output, &e)?;
    let hash = e.hash()?;
    tracing::info!("Election hash: {}", hex::encode(hash));
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use crate::create_election_file;
    use hex_literal::hex;

    use super::Config;

    const TEST_ELECTION_SEED: &str =
        "stool rich together paddle together pool raccoon promote attitude peasant latin concert";
    const TEST_ELECTION_HASH: &[u8] =
        &hex!("059f7f47936cbc080942035dded3f16d0e08b29347e08239dbba61c199de62f7");

    #[test]
    fn test_creator() -> anyhow::Result<()> {
        let c = Config {
            election_file: "../tests/test_election.yml".to_string(),
            seed: TEST_ELECTION_SEED.to_string(),
            output_file: "test_election.json".to_string(),
        };
        let h = create_election_file(&c)?;
        assert_eq!(h, TEST_ELECTION_HASH);
        Ok(())
    }
}
