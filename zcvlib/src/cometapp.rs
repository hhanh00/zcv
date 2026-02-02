use std::sync::Arc;

use anyhow::Result;
use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;
use tokio::{sync::Mutex, task::LocalSet};
use tonic::transport::Server;
use zcvlib::{
    ZCVError, context::BFTContext, db::create_schema, server::{rpc::ZCVServer, run_cometbft_app}, vote_rpc::vote_streamer_server::VoteStreamerServer
};

#[derive(Deserialize)]
pub struct Config {
    pub cometrpc_port: u16,
    pub cometbft_port: u16,
    pub grpc_port: u16,
    pub db_path: String,
    pub lwd_url: String,
    pub hash: String,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let config: Config = Figment::new().merge(Toml::file("zcv.toml")).extract()?;
    let Config {
        cometrpc_port,
        cometbft_port,
        grpc_port,
        db_path,
        lwd_url,
        hash,
    } = config;
    let hash = hex::decode(&hash)?;
    let context = BFTContext::new(&db_path, &lwd_url, cometrpc_port).await?;
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
            run_cometbft_app(context, &hash, cometbft_port)
                .await
                .unwrap();
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
                    let addr = format!("127.0.0.1:{}", grpc_port).parse().unwrap();
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
