use crate::database::{DatabaseHandle, Result, SqlError, SqlErrorKind};
use std::{collections::BTreeMap, path::Path};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i64,
    pub name: String,
    pub up: String,
    pub down: Option<String>,
}

impl Migration {
    pub fn new(version: i64, name: impl Into<String>, up: impl Into<String>) -> Self {
        Self {
            version,
            name: name.into(),
            up: up.into(),
            down: None,
        }
    }

    pub fn with_down(mut self, down: impl Into<String>) -> Self {
        self.down = Some(down.into());
        self
    }

    pub fn parse_filename(filename: &str) -> Option<(i64, String)> {
        let stem = filename.strip_suffix(".sql")?;
        let (version_str, name) = stem.split_once('_')?;
        let version = version_str.parse().ok()?;

        Some((version, name.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct MigrationRegistry {
    migrations: BTreeMap<i64, Migration>,
}

impl MigrationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let mut entries = fs::read_dir(dir).await.map_err(|e| {
            SqlError::new(
                SqlErrorKind::Query,
                format!("Failed to read migrations dir: {}", e),
            )
        })?;

        let mut result = Self {
            migrations: BTreeMap::new(),
        };

        let mut up_files: BTreeMap<i64, (String, String)> = BTreeMap::new();
        let mut down_files: BTreeMap<i64, String> = BTreeMap::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            SqlError::new(
                SqlErrorKind::Query,
                format!("Failed to read dir entry: {}", e),
            )
        })? {
            let path = entry.path();
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            if !filename.ends_with(".sql") {
                continue;
            }

            let content = fs::read_to_string(&path).await.map_err(|e| {
                SqlError::new(
                    SqlErrorKind::Query,
                    format!("Failed to read sql file: {}", e),
                )
            })?;

            if filename.contains(".up.sql") {
                let base_name = filename.replace(".up.sql", ".sql");
                if let Some((version, name)) = Migration::parse_filename(&base_name) {
                    up_files.insert(version, (name, content));
                }
            } else if filename.contains(".down.sql") {
                let base_name = filename.replace(".down.sql", ".sql");
                if let Some((version, _)) = Migration::parse_filename(&base_name) {
                    down_files.insert(version, content);
                }
            }
        }

        for (version, (name, up)) in up_files {
            let mut migration = Migration::new(version, name, up);
            if let Some(down) = down_files.remove(&version) {
                migration = migration.with_down(down);
            }

            result.register(migration);
        }

        Ok(result)
    }

    pub fn register(&mut self, migration: Migration) {
        self.migrations.insert(migration.version, migration);
    }

    pub fn get(&self, version: i64) -> Option<&Migration> {
        self.migrations.get(&version)
    }

    pub fn all(&self) -> impl Iterator<Item = &Migration> {
        self.migrations.values()
    }

    pub fn range(&self, from: i64, to: i64) -> impl Iterator<Item = &Migration> {
        self.migrations.range(from..=to).map(|(_, m)| m)
    }

    pub fn after(&self, version: i64) -> impl Iterator<Item = &Migration> {
        self.migrations.range((version + 1)..).map(|(_, m)| m)
    }

    pub fn after_rev(&self, version: i64) -> impl Iterator<Item = &Migration> {
        self.migrations.range((version + 1)..).rev().map(|(_, m)| m)
    }

    pub fn up_to(&self, version: i64) -> impl Iterator<Item = &Migration> {
        self.migrations.range(..=version).map(|(_, m)| m)
    }

    pub fn up_to_rev(&self, version: i64) -> impl Iterator<Item = &Migration> {
        self.migrations.range(..=version).rev().map(|(_, m)| m)
    }
}

#[derive(Debug)]
pub struct MigrationReport {
    pub initial_version: i64,
    pub target_version: i64,
    pub final_version: i64,
    pub applied: Vec<i64>,
    pub reverted: Vec<i64>,
}

#[derive(Debug)]
pub enum ValidationIssue {
    MissingMigration {
        version: i64,
        name: String,
    },
    NameMismatch {
        version: i64,
        expected: String,
        found: String,
    },
}

#[derive(Debug)]
pub struct MigrationRecord {
    pub version: i64,
    pub name: String,
    pub applied_at: chrono::DateTime<chrono::Utc>,
}

pub struct Migrator<'a> {
    db: &'a DatabaseHandle,
    registry: &'a MigrationRegistry,
    table_name: String,
}

impl<'a> Migrator<'a> {
    pub fn new(db: &'a DatabaseHandle, registry: &'a MigrationRegistry) -> Self {
        Self {
            db,
            registry,
            table_name: "_migrations".to_string(),
        }
    }

    pub fn with_table_name(mut self, name: impl Into<String>) -> Self {
        self.table_name = name.into();
        self
    }

    pub async fn init(&self) -> Result<()> {
        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                version BIGINT PRIMARY KEY,
                name TEXT NOT NULL
                applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name
        );

        self.db.execute(&sql, &[]).await?;
        Ok(())
    }

    pub async fn records(&self) -> Result<Vec<MigrationRecord>> {
        let sql = format!(
            "SELECT version, name, applied_at FROM {} ORDER BY version",
            self.table_name
        );

        let rows = self.db.query(&sql, &[]).await?;
        Ok(rows
            .iter()
            .map(|row| MigrationRecord {
                version: row.get("version"),
                name: row.get("name"),
                applied_at: row.get("applied_at"),
            })
            .collect())
    }

    pub async fn current_version(&self) -> Result<Option<i64>> {
        let sql = format!("SELECT MAX(version) as version FROM {}", self.table_name);
        let rows = self.db.query(&sql, &[]).await?;

        Ok(rows.first().and_then(|r| r.get("version")))
    }

    pub async fn pending(&self) -> Result<Vec<&Migration>> {
        let current = self.current_version().await?.unwrap_or(0);
        Ok(self
            .registry
            .after(current - 1)
            .filter(|m| m.version > current)
            .collect())
    }

    pub async fn migrate_pending(&self) -> Result<MigrationReport> {
        todo!();
    }

    pub async fn migrate_to(&self, target: i64) -> Result<MigrationReport> {
        todo!();
    }

    async fn migrate_up(&self, current: i64, target: i64) -> Result<MigrationReport> {
        todo!();
    }

    async fn migrate_down(&self, current: i64, target: i64) -> Result<MigrationReport> {
        todo!();
    }

    async fn apply_migration(&self, migration: &Migration) -> Result<()> {
        todo!();
    }

    async fn revert_migration(&self, migration: &Migration) -> Result<()> {
        todo!();
    }

    fn split_statements(sql: &str) -> Vec<&str> {
        sql.split(';').filter(|s| !s.trim().is_empty()).collect()
    }
}
