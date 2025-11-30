use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};
use tokio::time::Instant;
use tokio_postgres::Statement;

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

// Represents the hashed value of a sql query to be used
// as an entry for a cached sql statement hash map
#[derive(Clone, Eq, PartialEq, Hash)]
struct PreparedStatementKey(u64);

impl PreparedStatementKey {
    fn new(sql: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        sql.hash(&mut hasher);

        Self(hasher.finish())
    }
}

struct PreparedStatementCacheEntry {
    statement: Statement,
    sql: String,
    last_used: Instant,
    use_count: u64,
}

pub struct PreparedStatementCache {
    entries: HashMap<PreparedStatementKey, PreparedStatementCacheEntry>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl PreparedStatementCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            capacity,
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, sql: &str) -> Option<&Statement> {
        let key = PreparedStatementKey::new(sql);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = Instant::now();
            entry.use_count += 1;
            self.hits += 1;

            Some(&entry.statement)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, sql: &str, statement: Statement) {
        if self.entries.len() >= self.capacity {
            self.evict_lru();
        }

        let key = PreparedStatementKey::new(sql);
        self.entries.insert(
            key,
            PreparedStatementCacheEntry {
                statement,
                sql: sql.to_string(),
                last_used: Instant::now(),
                use_count: 1,
            },
        );
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.entries.len(),
            capacity: self.capacity,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    fn evict_lru(&mut self) {
        let to_remove = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone());

        if let Some(key) = to_remove {
            self.entries.remove(&key);
        }
    }
}
