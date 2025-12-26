use anyhow::Result;
use rocket::Config;
use zcvlib::{
    context::Context,
    server::{run_cometbft_app, run_rocket_server},
};

#[tokio::main]
pub async fn main() -> Result<()> {
    let config = Config::figment();
    let cometbft_port: u16 = config.extract_inner("custom.cometbft_port")?;
    let db_path: String = config.extract_inner("custom.db_path")?;
    let lwd_url: String = config.extract_inner("custom.lwd_url")?;
    let context = Context::new(&db_path, &lwd_url).await?;

    std::thread::spawn(move || run_cometbft_app(cometbft_port));
    let rest = std::thread::spawn(move || run_rocket_server(config, context));

    rest.join().unwrap()?;
    Ok(())
}
