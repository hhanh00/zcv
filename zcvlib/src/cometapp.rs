use std::sync::Arc;

use anyhow::Result;
use rocket::Config;
use tokio::{sync::Mutex, task::LocalSet};
use tonic::transport::Server;
use zcvlib::{
    ZCVError,
    context::Context,
    server::{rpc::ZCVServer, run_cometbft_app},
    vote_rpc::vote_streamer_server::VoteStreamerServer,
};

#[tokio::main]
pub async fn main() -> Result<()> {
    let config = Config::figment();
    let cometrpc_port: u16 = config.extract_inner("custom.cometrpc_port")?;
    let cometbft_port: u16 = config.extract_inner("custom.cometbft_port")?;
    let grpc_port: u16 = config.extract_inner("custom.grpc_port")?;
    let db_path: String = config.extract_inner("custom.db_path")?;
    let lwd_url: String = config.extract_inner("custom.lwd_url")?;
    let hash: String = config.extract_inner("custom.election")?;
    let hash = hex::decode(&hash)?;
    let context = Context::new(&db_path, &lwd_url, cometrpc_port).await?;
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
