use anyhow::Result;
use flutter_rust_bridge::frb;
use sqlx::SqliteConnection;

use crate::APPSTATE;

#[frb]
pub async fn put_prop(    key: &str,
    value: &str,
) -> Result<()> {
    let mut connection = APPSTATE.lock().await.connect().await?;
    put_prop_impl(&mut connection, key, value).await?;
    Ok(())
}

async fn put_prop_impl(
    connection: &mut SqliteConnection,
    key: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO props (key, value)
        VALUES (?, ?)
        ON CONFLICT(key) DO UPDATE SET value=excluded.value;
        "#,
    )
    .bind(key)
    .bind(value)
    .execute(&mut *connection)
    .await?;
    Ok(())
}
