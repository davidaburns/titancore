use anyhow::Result;
use sqlx::{PgPool, Postgres, postgres::PgPoolOptions};
use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

use crate::database::DbError;

pub struct DatabaseHandle {
    pool: PgPool,
    default_timeout: Duration,
    shutdown_sem: Arc<Semaphore>,
}

impl DatabaseHandle {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .test_before_acquire(true)
            .connect(database_url)
            .await
            .map_err(|e| DbError::PoolError(e.to_string()))?;

        Ok(Self {
            pool,
            default_timeout: Duration::from_secs(30),
            shutdown_sem: Arc::new(Semaphore::new(1000)),
        })
    }

    // pub async fn execute(
    //     &self,
    //     query: &str,
    //     params: &[&(dyn sqlx::Encode<Postgres> + Sync)],
    // ) -> Result<u64> {

    // }
    //
    // pub async fn execute_with_timeout(
    //     &self,
    //     query: &str,
    //     params: &[&(dyn sqlx::Encode<Postgres> + Sync)],
    //     timeout: Duration,
    // ) -> Result<u64> {
    //     let _permit = self.shutdown_sem.acquire().await.unwrap();
    // }
    //
    async fn execute_with_panic_recovery(
        &self,
        query: &str,
        params: &[&(dyn sqlx::Encode<Postgres> + Sync)],
        timeout: Duration,
    ) -> Result<sqlx::postgres::PgQueryResult> {
        let query_str = query.to_string();
        let params_debug: Vec<String> = params.iter().map(|_| "?".to_string()).collect();

        tokio::time::timeout(timeout, async {
            std::panic::catch_unwind(AssertUnwindSafe(|| async {
                let mut q = sqlx::query(query);
                for param in params {
                    q = q.bind(*param);
                }
                q.execute(&self.pool).await
            }))
            .unwrap_or_else(|panic_info| {
                Err(sqlx::Error::Protocol(format!(
                    "Panic: {:?}",
                    panic_info.downcast_ref::<&str>()
                )))
            })
            .await
        })
        .await
        .map_err(|_| DbError::Timeout(timeout))?
        .map_err(|e| DbError::QueryError {
            query: query_str,
            params: params_debug,
            source: e,
        })
    }
}
