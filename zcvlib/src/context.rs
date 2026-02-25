
use std::time::Duration;

use anyhow::Result;
use sqlx::{Sqlite, SqlitePool, pool::PoolConnection, sqlite::SqliteConnectOptions};

use crate::db::create_schema;

use crate::ZCVResult;

// #[frb(opaque)]
#[derive(Clone)]
pub struct Context {
    // #[flutter_rust_bridge::frb(ignore)]
    pub pool: SqlitePool,
    pub lwd_url: String,
    pub election_url: String,
}

impl Context {
    pub async fn new(db_path: &str, lwd_url: &str, election_url: &str) -> Result<Context> {
        let connect_options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .busy_timeout(Duration::from_mins(1))
            .filename(db_path);
        let pool = SqlitePool::connect_with(connect_options).await?;
        let mut conn = pool.acquire().await?;
        create_schema(&mut conn).await?;
        Ok(Context {
            pool,
            lwd_url: lwd_url.to_string(),
            election_url: election_url.to_string(),
        })
    }

    // #[flutter_rust_bridge::frb(ignore)]
    pub async fn connect(&self) -> Result<PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }
}

#[derive(Clone)]
pub struct BFTContext {
    pub context: Context,
    pub cometrpc_port: u16,
    pub grpc_port: u16,
}

impl BFTContext {
    pub fn init_logger() {
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .compact()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    pub async fn new(db_path: &str, lwd_url: &str, comet_rpcport: u16) -> ZCVResult<BFTContext> {
        Self::init_logger();
        let context = Context::new(db_path, lwd_url, "").await?;
        Ok(BFTContext {
            context,
            cometrpc_port: comet_rpcport,
            grpc_port: 0,
        })
    }

    pub async fn connect(&self) -> ZCVResult<PoolConnection<Sqlite>> {
        Ok(self.context.pool.acquire().await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::context::BFTContext;

    #[test]
    fn test_logger() {
        BFTContext::init_logger();
        tracing::info!("Test Logger");
    }
}
