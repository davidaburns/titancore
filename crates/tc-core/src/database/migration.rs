use std::collections::BTreeMap;

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

    pub fn register(&mut self, migration: Migration) {
        self.migrations.insert(migration.version, migration);
    }
}
