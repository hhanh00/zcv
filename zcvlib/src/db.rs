use sqlx::{SqliteConnection, query};

use crate::ZCVResult;

pub async fn create_schema(conn: &mut SqliteConnection) -> ZCVResult<()> {
    query(
        "CREATE TABLE IF NOT EXISTS blocks(
        height INTEGER PRIMARY KEY,
        data BLOB NOT NULL)",
    )
    .execute(conn)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::context::Context;
    use anyhow::Result;

    #[tokio::test]
    async fn test_schema_creation() -> Result<()> {
        let ctx = Context::new("vote.db", "").await?;
        let mut conn = ctx.connect().await?;
        super::create_schema(&mut conn).await?;

        let (c,): (u32,) = sqlx::query_as(
            "SELECT 1 FROM sqlite_master WHERE type = 'table'
            AND name = 'blocks'")
            .fetch_one(&mut *conn)
            .await?;

        assert_eq!(c, 1);
        Ok(())
    }
}
