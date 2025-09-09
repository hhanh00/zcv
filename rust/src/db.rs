use anyhow::Result;
use sqlx::SqliteConnection;

pub async fn create_schema(connection: &mut SqliteConnection) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS props (
            id INTEGER PRIMARY KEY,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            UNIQUE(key)
        );
        "#,
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blocks (
            height INTEGER PRIMARY KEY,
            data BLOB NOT NULL
        );
        "#,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

