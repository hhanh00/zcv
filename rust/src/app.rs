use std::{path::PathBuf, str::FromStr};

use anyhow::Result;
use sqlx::{pool::PoolConnection, sqlite::SqliteConnectOptions, Sqlite, SqlitePool};
use tonic::transport::{Channel, ClientTlsConfig};
use tracing::info;

use crate::{db::create_schema, rpc::compact_tx_streamer_client::CompactTxStreamerClient, Client};

#[derive(Default)]
pub struct AppState {
    pub initialized: bool,
    pub lwd: String,
    pub db_path: String,
    pub pool: Option<SqlitePool>,
}

impl AppState {
    pub fn set_lwd(&mut self, lwd: &str) {
        self.lwd = lwd.to_string();
    }

    pub async fn init_db(&mut self, dir: &str, name: &str) -> Result<()> {
        self.db_path = dir.to_string();
        let mut path = PathBuf::from_str(dir)?;
        path.push(name);
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        self.pool = Some(SqlitePool::connect_with(options).await?);
        let mut connection = self.connect().await?;
        create_schema(&mut connection).await?;

        let mut lmdb_dir = PathBuf::from_str(dir)?;
        lmdb_dir.push("lmdb");
        let _ = std::fs::create_dir(&lmdb_dir);
        info!("{:?}", &lmdb_dir);
        Ok(())
    }

    pub async fn connect(&self) -> Result<PoolConnection<Sqlite>> {
        if let Some(pool) = &self.pool {
            Ok(pool.acquire().await?)
        } else {
            Err(anyhow::anyhow!("Database not initialized"))
        }
    }

    pub async fn client(&self) -> Result<Client> {
        let mut channel = Channel::from_shared(self.lwd.clone())?;
        if self.lwd.starts_with("https") {
            let tls = ClientTlsConfig::new().with_enabled_roots();
            channel = channel.tls_config(tls)?;
        }
        let client = CompactTxStreamerClient::connect(channel).await?;
        Ok(client)
    }
}
