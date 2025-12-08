use std::time::Duration;
use tokio::{sync::RwLock, time::timeout};
use tokio_postgres::{Row, Statement, Transaction as PgTransaction};

use crate::database::{
    QueryParam, Result, SqlError, SqlErrorKind, SqlResultExt,
    cache::{CacheStats, PreparedStatementCache},
};

pub struct TransactionContext<'a> {
    pub tx: &'a PgTransaction<'a>,
    pub cache: &'a RwLock<PreparedStatementCache>,
    pub query_timeout: Duration,
}

impl<'a> TransactionContext<'a> {
    pub async fn query(&self, sql: &str, params: &[&QueryParam]) -> Result<Vec<Row>> {
        let stmt = self.prepare_cached(sql).await?;
        timeout(self.query_timeout, self.tx.query(&stmt, params))
            .await
            .map_err(|_| SqlError::new(SqlErrorKind::Timeout, "Transaction query timed out"))?
            .sql_err(SqlErrorKind::Query)
            .map_err(|e| e.query(sql).context("Transaction query failed"))
    }

    pub async fn query_single(&self, sql: &str, params: &[&QueryParam]) -> Result<Row> {
        let rows = self.query(sql, params).await?;
        match rows.len() {
            0 => Err(
                SqlError::new(SqlErrorKind::Query, "Expected a single row, got nonw").query(sql),
            ),
            1 => Ok(rows.into_iter().next().unwrap()),
            n => Err(SqlError::new(
                SqlErrorKind::Query,
                format!("Expected a single row, got {}", n),
            )
            .query(sql)),
        }
    }

    pub async fn query_scalar<T>(&self, sql: &str, params: &[&QueryParam]) -> Result<T>
    where
        T: for<'r> tokio_postgres::types::FromSql<'r>,
    {
        let row = self.query_single(sql, params).await?;
        row.try_get(0).map_err(|e| {
            SqlError::with_source(SqlErrorKind::Query, e)
                .query(sql)
                .context("Failed to extract scalar value")
        })
    }

    pub async fn execute(&self, sql: &str, params: &[&QueryParam]) -> Result<u64> {
        let stmt = self.prepare_cached(sql).await?;
        timeout(self.query_timeout, self.tx.execute(&stmt, params))
            .await
            .map_err(|_| SqlError::new(SqlErrorKind::Timeout, "Transaction execute timed out"))?
            .sql_err(SqlErrorKind::Query)
            .map_err(|e| e.query(sql).context("Transaction execute failed"))
    }

    pub async fn cache_stats(&self) -> CacheStats {
        self.cache.read().await.stats()
    }

    async fn prepare_cached(&self, sql: &str) -> Result<Statement> {
        {
            let mut cache = self.cache.write().await;
            if let Some(stmt) = cache.get(sql) {
                return Ok(stmt.clone());
            }
        }

        let stmt = timeout(self.query_timeout, self.tx.prepare(sql))
            .await
            .map_err(|_| SqlError::new(SqlErrorKind::Timeout, "Prepare timed out in transaction"))?
            .sql_err(SqlErrorKind::Query)
            .map_err(|e| e.query(sql).context("Failed to prepare in transaction"))?;

        self.cache.write().await.insert(sql, stmt.clone());
        Ok(stmt)
    }
}
