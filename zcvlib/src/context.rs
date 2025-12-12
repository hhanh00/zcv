use sqlx::{ConnectOptions, Sqlite, SqliteConnection, SqlitePool, pool::PoolConnection, sqlite::SqliteConnectOptions};

use crate::{ZCVResult, error::IntoAnyhow};

pub struct Context {
    pub pool: SqlitePool,
    pub lwd_url: String,
}

impl Context {
    pub async fn new(db_path: &str, lwd_url: &str) -> ZCVResult<Context> {
        let connect_options = SqliteConnectOptions::new()
        .create_if_missing(true)
        .filename(db_path);
        let pool = SqlitePool::connect_with(connect_options).await?;
        Ok(Context {
            pool,
            lwd_url: lwd_url.to_string(),
        })
    }

    pub async fn connect(&self) -> ZCVResult<PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }
}
