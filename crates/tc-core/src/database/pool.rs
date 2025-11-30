use crate::database::{ConnectionGuard, PooledConnection, Result, SqlError, SqlResultExt};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Mutex, Notify, Semaphore},
    time::{Instant, timeout},
};
use tokio_postgres::{Config, NoTls};

#[derive(Clone)]
pub struct PoolConfig {
    pub connection_string: String,
    pub min_connections: usize,
    pub max_connection: usize,
    pub acquire_timeout: Duration,
    pub query_timeout: Duration,
    pub health_check_interval: Duration,
    pub idle_timeout: Duration,
    pub statement_cache_capacity: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            connection_string: String::new(),
            min_connections: 2,
            max_connection: 10,
            acquire_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            statement_cache_capacity: 100,
        }
    }
}

#[derive(Debug)]
pub struct ConnectionPoolStats {
    pub active: usize,
    pub total_created: usize,
    pub is_shutdown: bool,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
}

pub struct ConnectionPool {
    config: PoolConfig,
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    sem: Arc<Semaphore>,
    shutdown: AtomicBool,
    shutdown_notify: Notify,
    active_count: AtomicUsize,
    total_created: AtomicUsize,
    total_cache_hits: AtomicU64,
    total_cache_misses: AtomicU64,
}

impl ConnectionPool {
    pub async fn new(config: PoolConfig) -> Result<Arc<Self>> {
        let pool = Arc::new(Self {
            sem: Arc::new(Semaphore::new(config.max_connection)),
            config,
            connections: Arc::new(Mutex::new(VecDeque::new())),
            shutdown: AtomicBool::new(false),
            shutdown_notify: Notify::new(),
            active_count: AtomicUsize::new(0),
            total_created: AtomicUsize::new(0),
            total_cache_hits: AtomicU64::new(0),
            total_cache_misses: AtomicU64::new(0),
        });

        for i in 0..pool.config.min_connections {
            let conn = pool
                .create_connection()
                .await
                .map_err(|e| e.context(format!("Failed to create initial connection {}", i)))?;

            pool.connections.lock().await.push_back(conn);
        }

        let pool_clone = Arc::clone(&pool);
        tokio::spawn(async move {
            pool_clone.health_check_loop().await;
        });

        Ok(pool)
    }

    pub async fn acquire(&'_ self) -> Result<ConnectionGuard<'_>> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SqlError::new(
                super::SqlErrorKind::Shutdown,
                "Pool is shutting down",
            ));
        }

        let permit = timeout(
            self.config.acquire_timeout,
            self.sem.clone().acquire_owned(),
        )
        .await
        .map_err(|_| {
            SqlError::new(
                super::SqlErrorKind::Timeout,
                "Timed out waiting for connection",
            )
        })?
        .map_err(|_| SqlError::new(super::SqlErrorKind::Pool, "Connection semaphore closed"))?;

        let mut conn = self.connections.lock().await.pop_front();
        if conn.is_none() {
            conn = Some(
                self.create_connection()
                    .await
                    .map_err(|e| e.context("Failed to create new pooled connection"))?,
            );
        }

        let mut conn = conn.unwrap();
        conn.touch();

        self.active_count.fetch_add(1, Ordering::Relaxed);
        Ok(ConnectionGuard::new(Some(conn), self, permit))
    }

    pub fn return_connection(&self, conn: PooledConnection) {
        let stats = conn.cache_states();
        self.total_cache_hits
            .fetch_add(stats.hits, Ordering::Relaxed);

        self.total_cache_misses
            .fetch_add(stats.misses, Ordering::Relaxed);

        self.active_count.fetch_sub(1, Ordering::Relaxed);

        if self.shutdown.load(Ordering::Acquire) {
            return;
        }

        let pool_connections = self.connections.clone();
        tokio::spawn(async move {
            pool_connections.lock().await.push_back(conn);
        });
    }

    pub async fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.shutdown_notify.notify_waiters();

        let deadline = Instant::now() + Duration::from_secs(30);
        while self.active_count.load(Ordering::Relaxed) > 0 {
            if Instant::now() > deadline {
                tracing::error!(
                    "Shutdown timeout: {} connections still active",
                    self.active_count.load(Ordering::Relaxed)
                );

                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        self.connections.lock().await.clear();
    }

    async fn create_connection(&self) -> Result<PooledConnection> {
        let config: Config = self
            .config
            .connection_string
            .parse()
            .sql_err(super::SqlErrorKind::Connection)
            .map_err(|e| e.context("Invalid connection string"))?;

        let (client, connection) = timeout(self.config.acquire_timeout, config.connect(NoTls))
            .await
            .map_err(|_| {
                SqlError::new(super::SqlErrorKind::Timeout, "Connection attempt timed out")
            })?
            .sql_err(super::SqlErrorKind::Connection)
            .map_err(|e| e.context("Failed to establish database connection"))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("Connection error: {}", e)
            }
        });

        self.total_created.fetch_add(1, Ordering::Relaxed);
        Ok(PooledConnection::new(
            client,
            self.config.statement_cache_capacity,
        ))
    }

    async fn health_check_loop(self: Arc<Self>) {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.config.health_check_interval) => {
                    self.run_health_check().await;
                }
                _ = self.shutdown_notify.notified() => break,
            }
        }
    }

    async fn run_health_check(&self) {
        let mut connections = self.connections.lock().await;
        let mut healthy = VecDeque::new();

        while let Some(mut conn) = connections.pop_front() {
            if healthy.len() >= self.config.min_connections
                && conn.is_past_idle_timeout(self.config.idle_timeout)
            {
                continue;
            }

            match conn.client.simple_query("SELECT 1").await {
                Ok(_) => {
                    conn.touch();
                    healthy.push_back(conn);
                }
                Err(e) => {
                    tracing::error!("Health check failed: {}", e);
                }
            }
        }

        *connections = healthy;
    }

    pub fn stats(&self) -> ConnectionPoolStats {
        let hits = self.total_cache_hits.load(Ordering::Relaxed);
        let misses = self.total_cache_misses.load(Ordering::Relaxed);

        ConnectionPoolStats {
            active: self.active_count.load(Ordering::Relaxed),
            total_created: self.total_created.load(Ordering::Relaxed),
            is_shutdown: self.shutdown.load(Ordering::Acquire),
            cache_hits: hits,
            cache_misses: misses,
            cache_hit_rate: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
        }
    }
}
