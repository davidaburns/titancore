use crate::database::{DatabaseHandle, Result, SqlError, SqlErrorKind};
use std::{cmp::Ordering, collections::BTreeMap, i64, path::Path};
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

impl MigrationReport {
    fn new(initial: i64, target: i64) -> Self {
        Self {
            initial_version: initial,
            target_version: target,
            final_version: initial,
            applied: Vec::new(),
            reverted: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.final_version == self.target_version
            || (self.target_version == i64::MAX
                && self.applied.is_empty()
                && self.reverted.is_empty())
    }

    pub fn changes(&self) -> usize {
        self.applied.len() + self.reverted.len()
    }
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

pub struct MigrationMigrator<'a> {
    db: &'a DatabaseHandle,
    registry: &'a MigrationRegistry,
    table_name: String,
}

impl<'a> MigrationMigrator<'a> {
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
                name TEXT NOT NULL,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name
        );

        self.db.execute(&sql, &[]).await?;
        Ok(())
    }

    pub async fn initialized(&self) -> Result<bool> {
        let sql = r#"
            SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema='public' AND table_name=$1
            )
        "#;

        let exists: bool = self.db.query_scalar(sql, &[&self.table_name]).await?;
        Ok(exists)
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
        self.migrate_to(i64::MAX).await
    }

    pub async fn migrate_to(&self, target: i64) -> Result<MigrationReport> {
        let current = self.current_version().await?.unwrap_or(0);
        match target.cmp(&current) {
            Ordering::Greater | Ordering::Equal => self.migrate_up(current, target).await,
            Ordering::Less => self.migrate_down(current, target).await,
        }
    }

    async fn migrate_up(&self, current: i64, target: i64) -> Result<MigrationReport> {
        let mut report = MigrationReport::new(current, target);
        let pending: Vec<_> = self
            .registry
            .all()
            .filter(|m| m.version > current && m.version <= target)
            .collect();

        for migration in pending {
            self.apply_migration(migration).await?;
            report.applied.push(migration.version);
        }

        report.final_version = self.current_version().await?.unwrap_or(0);
        Ok(report)
    }

    async fn migrate_down(&self, current: i64, target: i64) -> Result<MigrationReport> {
        let mut report = MigrationReport::new(current, target);
        let to_revert: Vec<_> = self
            .registry
            .all()
            .filter(|m| m.version <= current && m.version > target)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        for migration in to_revert {
            self.revert_migration(migration).await?;
            report.reverted.push(migration.version);
        }

        report.final_version = self.current_version().await?.unwrap_or(0);
        Ok(report)
    }

    async fn apply_migration(&self, migration: &Migration) -> Result<()> {
        tracing::info!(
            "Applying migration {}: {}",
            migration.version,
            migration.name
        );

        self.db
            .transaction(async |tx| {
                let version = migration.version;
                let name = migration.name.clone();
                let up_sql = migration.up.clone();
                let table = self.table_name.clone();

                for stmt in Self::split_statements(&up_sql) {
                    let stmt = stmt.trim();
                    if !stmt.is_empty() {
                        tx.execute(stmt, &[]).await?;
                    }
                }

                let migration_record_sql =
                    format!("INSERT INTO {} (version, name) VALUES ($1, $2);", table);

                tx.execute(&migration_record_sql, &[&version, &name])
                    .await?;

                Ok(())
            })
            .await
            .map_err(|e| e.context(format!("Migration {} failed", migration.version)))?;

        Ok(())
    }

    async fn revert_migration(&self, migration: &Migration) -> Result<()> {
        let down_sql = migration.down.as_ref().ok_or_else(|| {
            SqlError::new(
                SqlErrorKind::Query,
                format!("Migration {} has no down script", migration.version),
            )
        })?;

        tracing::info!(
            "Reverting migration {}: {}",
            migration.version,
            migration.name
        );

        self.db
            .transaction(async |tx| {
                let version = migration.version;
                let down_sql = down_sql.clone();
                let table = self.table_name.clone();

                for stmt in Self::split_statements(&down_sql) {
                    let stmt = stmt.trim();
                    if !stmt.is_empty() {
                        tx.execute(stmt, &[]).await?;
                    }
                }

                let migration_revert_sql = format!("DELETE FROM {} WHERE version = $1;", table);
                tx.execute(&migration_revert_sql, &[&version]).await?;

                Ok(())
            })
            .await
            .map_err(|e| e.context(format!("Revert {} failed", migration.version)))?;

        Ok(())
    }

    fn split_statements(sql: &str) -> Vec<&str> {
        sql.split(';').filter(|s| !s.trim().is_empty()).collect()
    }
}
