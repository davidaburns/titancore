use std::time::Duration;

use tokio::time::Instant;
use tokio_postgres::Client;

use crate::database::cache::StatementCache;

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

struct PooledConnection {
    client: Client,
    cache: StatementCache,
    created_at: Instant,
    last_used: Instant,
}

impl PooledConnection {}
