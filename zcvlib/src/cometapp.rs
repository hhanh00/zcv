use anyhow::Result;
use rocket::Config;
use zcvlib::{
    ZCVError, context::Context, server::{run_cometbft_app, run_rocket_server}
};

#[tokio::main]
pub async fn main() -> Result<()> {
    let config = Config::figment();
    let cometrpc_port: u16 = config.extract_inner("custom.cometrpc_port")?;
    let cometbft_port: u16 = config.extract_inner("custom.cometbft_port")?;
    let db_path: String = config.extract_inner("custom.db_path")?;
    let lwd_url: String = config.extract_inner("custom.lwd_url")?;
    let hash: String = config.extract_inner("custom.election")?;
    let hash = hex::decode(&hash)?;
    let context = Context::new(&db_path, &lwd_url).await?;
    let context2 = context.clone();

    std::thread::spawn(move || {
        let r = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        r.block_on(async move {
            run_cometbft_app(&context2, &hash, cometbft_port).await.unwrap();
            Ok::<_, ZCVError>(())
        })
    });
    let rest = std::thread::spawn(move || run_rocket_server(config, context, cometrpc_port));

    rest.join().unwrap()?;
    Ok(())
}
