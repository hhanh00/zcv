
use sqlx::{Sqlite, pool::PoolConnection};

use crate::{ZCVResult, api::Context};

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
