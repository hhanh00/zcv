use anyhow::Result;
use tracing::info;
use clap::Parser;
use figment::{Figment, providers::{Format, Yaml}};
use zcv_setup::config::{CLIArgs, Config};

fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .compact()
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let args = CLIArgs::parse();
    let config_path = args.config_path.unwrap_or("zcv-setup.yml".to_string());
    let config: Config = Figment::new().merge(Yaml::file(&config_path)).extract()?;
    info!("Cluster Config {}", serde_json::to_string_pretty(&config)?);

    Ok(())
}
