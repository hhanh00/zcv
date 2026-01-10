use std::time::Duration;

use sqlx::{Sqlite, SqlitePool, pool::PoolConnection, sqlite::SqliteConnectOptions};

use crate::ZCVResult;

#[derive(Clone)]
pub struct Context {
    pub pool: SqlitePool,
    pub lwd_url: String,
    pub comet_port: u16,
}

impl Context {
    pub fn init_logger() {
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .compact()
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    pub async fn new(db_path: &str, lwd_url: &str) -> ZCVResult<Context> {
        Self::init_logger();
        let connect_options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .busy_timeout(Duration::from_mins(1))
            .filename(db_path);
        let pool = SqlitePool::connect_with(connect_options).await?;
        Ok(Context {
            pool,
            lwd_url: lwd_url.to_string(),
            comet_port: 0,
        })
    }

    pub async fn connect(&self) -> ZCVResult<PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::context::Context;

    #[test]
    fn test_logger() {
        Context::init_logger();
        tracing::info!("Test Logger");
    }
}
