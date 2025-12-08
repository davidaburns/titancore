use crate::database::{
    ConnectionPool, ConnectionPoolStats, PoolConfig, Result, SqlError, SqlErrorKind, SqlResultExt,
    TransactionContext, cache::PreparedStatementCache,
};
use futures::FutureExt;
use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::timeout};
use tokio_postgres::{Row, types::ToSql};

pub type QueryParam = dyn ToSql + Sync;

pub struct DatabaseHandle {
    pool: Arc<ConnectionPool>,
    query_timeout: Duration,
}

impl DatabaseHandle {
    pub async fn connect(config: PoolConfig) -> Result<Self> {
        let timeout = config.query_timeout;
        let pool = ConnectionPool::new(config).await?;

        Ok(Self {
            pool,
            query_timeout: timeout,
        })
    }

    pub async fn query(&self, sql: &str, params: &[&QueryParam]) -> Result<Vec<Row>> {
        self.with_panic_recovery(sql, async {
            let mut conn = self.pool.acquire().await?;
            let stmt = conn
                .conn_mut()
                .prepare_cached(sql, self.query_timeout)
                .await?;

            timeout(self.query_timeout, conn.client().query(&stmt, params))
                .await
                .map_err(|_| {
                    SqlError::new(SqlErrorKind::Timeout, "Query execution timed out")
                        .query(sql)
                        .context(format!("timeout={:?}", self.query_timeout))
                })?
                .sql_err(SqlErrorKind::Query)
                .map_err(|e| e.query(sql).context("Query execution failed"))
        })
        .await
    }

    pub async fn query_single(&self, sql: &str, params: &[&QueryParam]) -> Result<Row> {
        let rows = self.query(sql, params).await?;
        match rows.len() {
            0 => Err(
                SqlError::new(SqlErrorKind::Query, "Expected a single row, got none").query(sql),
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
        T: for<'a> tokio_postgres::types::FromSql<'a>,
    {
        let row = self.query_single(sql, params).await?;
        row.try_get(0).map_err(|e| {
            SqlError::with_source(SqlErrorKind::Query, e)
                .query(sql)
                .context("Failed to extract scalar value")
        })
    }

    pub async fn execute(&self, sql: &str, params: &[&QueryParam]) -> Result<u64> {
        self.with_panic_recovery(sql, async {
            let mut conn = self.pool.acquire().await?;
            let stmt = conn
                .conn_mut()
                .prepare_cached(sql, self.query_timeout)
                .await?;

            timeout(self.query_timeout, conn.client().execute(&stmt, params))
                .await
                .map_err(|_| {
                    SqlError::new(SqlErrorKind::Timeout, "Statement execution timed out").query(sql)
                })?
                .sql_err(SqlErrorKind::Query)
                .map_err(|e| e.query(sql).context("Statement execution failed"))
        })
        .await
    }

    pub async fn query_unprepared(&self, sql: &str, params: &[&QueryParam]) -> Result<Vec<Row>> {
        self.with_panic_recovery(sql, async {
            let conn = self.pool.acquire().await?;
            let stmt = timeout(self.query_timeout, conn.client().prepare(sql))
                .await
                .map_err(|_| SqlError::new(SqlErrorKind::Timeout, "Prepare timed out"))?
                .sql_err(SqlErrorKind::Query)
                .map_err(|e| e.query(sql))?;

            timeout(self.query_timeout, conn.client().query(&stmt, params))
                .await
                .map_err(|_| SqlError::new(SqlErrorKind::Timeout, "Query timed out"))?
                .sql_err(SqlErrorKind::Query)
                .map_err(|e| e.query(sql))
        })
        .await
    }

    pub async fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: for<'c> AsyncFnOnce(TransactionContext<'c>) -> Result<T>,
    {
        self.with_panic_recovery("TRANSACTION", async {
            let mut conn = self.pool.acquire().await?;
            let tx = conn
                .client_mut()
                .transaction()
                .await
                .sql_err(SqlErrorKind::Transaction)
                .map_err(|e| e.context("Failed to begin transaction"))?;

            let tx_cache = RwLock::new(PreparedStatementCache::new(
                self.pool.config.statement_cache_capacity,
            ));

            let ctx = TransactionContext {
                tx: &tx,
                cache: &tx_cache,
                query_timeout: self.query_timeout,
            };

            match f(ctx).await {
                Ok(result) => {
                    tx.commit()
                        .await
                        .sql_err(SqlErrorKind::Transaction)
                        .map_err(|e| e.context("Failed to commit transaction"))?;

                    Ok(result)
                }
                Err(e) => Err(e.context("Transaction rolled back")),
            }
        })
        .await
    }

    pub async fn shutdown(&self) {
        self.pool.shutdown().await;
    }

    pub fn stats(&self) -> ConnectionPoolStats {
        self.pool.stats()
    }

    async fn with_panic_recovery<F, T>(&self, context: &str, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        match AssertUnwindSafe(f).catch_unwind().await {
            Ok(result) => result,
            Err(panic) => {
                let msg = panic
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "Unknown panic".to_string());

                Err(
                    SqlError::new(SqlErrorKind::Panic, format!("Panic recovered: {}", msg))
                        .query(context),
                )
            }
        }
    }
}
