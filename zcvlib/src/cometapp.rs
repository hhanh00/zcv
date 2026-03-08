use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use figment::{
    Figment,
    providers::{Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::LocalSet};
use tonic::transport::Server;
use zcvlib::{
    ZCVError,
    context::BFTContext,
    db::create_schema,
    server::{rpc::ZCVServer, run_cometbft_app},
    vote::VK,
    vote_rpc::vote_streamer_server::VoteStreamerServer,
};

#[derive(Parser, Serialize, Deserialize, Debug)]
pub struct Config {
    #[clap(short = 'r', long, value_parser)]
    pub cometrpc_port: Option<u16>,
    #[clap(short = 'b', long, value_parser)]
    pub cometbft_port: Option<u16>,
    #[clap(short = 'g', long, value_parser)]
    pub grpc_port: Option<u16>,
    #[clap(short, long, value_parser)]
    pub db_path: Option<String>,
    #[clap(short, long, value_parser)]
    pub lwd_url: Option<String>,
    #[clap(short, long)]
    pub unsafe_skip_validation: bool,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    std::sync::LazyLock::force(&VK);

    let config = Config::parse();
    let config: Config = Figment::new()
        .merge(Toml::file("zcv.toml"))
        .join(Serialized::defaults(config))
        .extract()?;
    let Config {
        cometrpc_port,
        cometbft_port,
        grpc_port,
        db_path,
        lwd_url,
        unsafe_skip_validation,
    } = config;
    let cometrpc_port = cometrpc_port.unwrap_or(26657);
    let cometbft_port = cometbft_port.unwrap_or(26658);
    let grpc_port = grpc_port.unwrap_or(9010);
    let db_path = db_path.unwrap_or("vote.db".to_string());
    let lwd_url = lwd_url.unwrap_or("https://zec.rocks".to_string());

    let context =
        BFTContext::new(&db_path, &lwd_url, cometrpc_port, unsafe_skip_validation).await?;
    {
        let mut conn = context.connect().await?;
        create_schema(&mut conn).await?;
    }

    let context = Arc::new(Mutex::new(context));
    let context2 = context.clone();

    std::thread::spawn(move || {
        let r = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        r.block_on(async move {
            run_cometbft_app(context, cometbft_port).await.unwrap();
            Ok::<_, ZCVError>(())
        })
    });

    let grpc_server = std::thread::spawn(move || {
        let r = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        r.block_on(async move {
            let local = LocalSet::new();
            local
                .run_until(async move {
                    let service = ZCVServer { context: context2 };
                    let addr = format!("0.0.0.0:{}", grpc_port).parse().unwrap();
                    let mut builder = Server::builder();
                    builder
                        .add_service(VoteStreamerServer::new(service))
                        .serve(addr)
                        .await
                })
                .await?;
            Ok::<_, anyhow::Error>(())
        })?;
        Ok::<_, anyhow::Error>(())
    });

    grpc_server.join().unwrap()?;
    Ok(())
}
