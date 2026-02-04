use std::time::Duration;

use anyhow::Result;
use flutter_rust_bridge::frb;
use sqlx::{Sqlite, SqlitePool, pool::PoolConnection, sqlite::SqliteConnectOptions};

use crate::db::create_schema;

pub mod init;
pub mod simple;


#[frb(opaque)]
#[derive(Clone)]
pub struct Context {
    #[frb(ignore)]
    pub pool: SqlitePool,
    pub lwd_url: String,
    pub election_url: String,
}

impl Context {
    #[frb(ignore)]
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

    #[frb(ignore)]
    pub async fn connect(&self) -> Result<PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }
}
