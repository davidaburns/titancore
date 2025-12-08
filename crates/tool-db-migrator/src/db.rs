use tc_core::database::{DatabaseHandle, PoolConfig, Result, SqlErrorKind, SqlResultExt};
use url::Url;

pub fn database_from_connection_string(conn: &String) -> Result<String> {
    let url = Url::parse(&conn)
        .sql_err(SqlErrorKind::Connection)
        .map_err(|e| e.context("Failed to parse connection string"))?;

    let path = url.path();
    let extracted = if path.len() > 1 {
        &path.to_string()[1..]
    } else {
        ""
    };

    Ok(extracted.to_string())
}

pub async fn database_exists(conn: &String, db_name: &String) -> Result<bool> {
    let mut url = Url::parse(&conn)
        .sql_err(SqlErrorKind::Connection)
        .map_err(|e| e.context("Failed to parse connection string"))?;

    url.set_path("postgres");
    let config = PoolConfig {
        connection_string: url.to_string(),
        ..Default::default()
    };

    let db = DatabaseHandle::connect(config).await?;
    let exists: bool = db
        .query_scalar(
            "SELECT EXISTS(SELECT 1 as exists FROM pg_database WHERE datname=$1);",
            &[db_name],
        )
        .await?;

    Ok(exists)
}

pub async fn create_database(conn: &String, db_name: &String) -> Result<()> {
    let mut url = Url::parse(&conn)
        .sql_err(SqlErrorKind::Connection)
        .map_err(|e| e.context("Failed to parse connection string"))?;

    url.set_path("postgres");
    let config = PoolConfig {
        connection_string: url.to_string(),
        ..Default::default()
    };

    let db = DatabaseHandle::connect(config).await?;
    let sql = format!(r#"CREATE DATABASE {};"#, db_name);
    db.execute(&sql, &[]).await?;

    Ok(())
}
