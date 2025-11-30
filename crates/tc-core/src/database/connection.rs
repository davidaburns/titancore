use crate::database::{
    ConnectionPool, Result, SqlError, SqlResultExt,
    cache::{CacheStats, PreparedStatementCache},
};
use std::time::Duration;
use tokio::time::{Instant, timeout};
use tokio_postgres::{Client, Statement};

pub struct PooledConnection {
    pub client: Client,
    cache: PreparedStatementCache,
    created_at: Instant,
    last_used: Instant,
}

impl PooledConnection {
    pub fn new(client: Client, cache_capacity: usize) -> Self {
        let now = Instant::now();
        Self {
            client,
            cache: PreparedStatementCache::new(cache_capacity),
            created_at: now,
            last_used: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    pub fn is_past_idle_timeout(&self, timeout: Duration) -> bool {
        self.last_used.elapsed() > timeout
    }

    pub async fn prepare_cached(
        &mut self,
        sql: &str,
        query_timeout: Duration,
    ) -> Result<Statement> {
        if let Some(stmt) = self.cache.get(sql) {
            return Ok(stmt.clone());
        }

        let stmt = timeout(query_timeout, self.client.prepare(sql))
            .await
            .map_err(|_| SqlError::new(super::SqlErrorKind::Timeout, "Prepare statement timedout"))?
            .sql_err(super::SqlErrorKind::Query)
            .map_err(|e| e.query(sql).context("Failed to prepare statement"))?;

        self.cache.insert(sql, stmt.clone());
        Ok(stmt)
    }

    pub fn cache_states(&self) -> CacheStats {
        self.cache.stats()
    }
}

pub struct ConnectionGuard<'a> {
    pub conn: Option<PooledConnection>,
    pub pool: &'a ConnectionPool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl<'a> ConnectionGuard<'a> {
    pub fn new(
        conn: Option<PooledConnection>,
        pool: &'a ConnectionPool,
        permit: tokio::sync::OwnedSemaphorePermit,
    ) -> Self {
        Self {
            conn,
            pool,
            _permit: permit,
        }
    }

    pub fn client(&self) -> &Client {
        &self.conn.as_ref().unwrap().client
    }

    pub fn client_mut(&mut self) -> &mut Client {
        &mut self.conn.as_mut().unwrap().client
    }

    pub fn conn_mut(&mut self) -> &mut PooledConnection {
        self.conn.as_mut().unwrap()
    }
}

impl Drop for ConnectionGuard<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.return_connection(conn);
        }
    }
}
