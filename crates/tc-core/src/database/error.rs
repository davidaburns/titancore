#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlErrorKind {
    Connection,
    Timeout,
    Query,
    Transaction,
    Pool,
    HealthCheck,
    Panic,
    Shutdown,
}

#[derive(Debug)]
pub struct SqlError {
    pub kind: SqlErrorKind,
    pub query: Option<String>,
    source: anyhow::Error,
}

impl SqlError {
    pub fn new(kind: SqlErrorKind, msg: impl Into<String>) -> Self {
        Self {
            kind,
            query: None,
            source: anyhow::anyhow!(msg.into()),
        }
    }

    pub fn with_source(
        kind: SqlErrorKind,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            query: None,
            source: anyhow::Error::new(source),
        }
    }

    pub fn query(mut self, sql: impl Into<String>) -> Self {
        self.query = Some(sql.into());
        self
    }

    pub fn context(mut self, ctx: impl std::fmt::Display + Send + Sync + 'static) -> Self {
        self.source = self.source.context(ctx.to_string());
        self
    }

    pub fn chain(&self) -> impl Iterator<Item = &(dyn std::error::Error + 'static)> {
        self.source.chain()
    }
}

impl std::fmt::Display for SqlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}]", self.kind)?;
        if let Some(q) = &self.query {
            let q = if q.len() > 100 { &q[..100] } else { q };
            write!(f, " queryy={}", q)?;
        }

        write!(f, " {}", self.source)
    }
}

impl std::error::Error for SqlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.source()
    }
}

pub type Result<T> = std::result::Result<T, SqlError>;

pub trait SqlResultExt<T> {
    fn sql_err(self, kind: SqlErrorKind) -> Result<T>;
    fn with_query(self, sql: &str) -> Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> SqlResultExt<T>
    for std::result::Result<T, E>
{
    fn sql_err(self, kind: SqlErrorKind) -> Result<T> {
        self.map_err(|e| SqlError::with_source(kind, e))
    }

    fn with_query(self, sql: &str) -> Result<T> {
        self.map_err(|e| SqlError::with_source(SqlErrorKind::Query, e).query(sql))
    }
}

pub trait SqlOptionExt<T> {
    fn sql_ok_or(self, kind: SqlErrorKind, msg: &str) -> Result<T>;
}

impl<T> SqlOptionExt<T> for Option<T> {
    fn sql_ok_or(self, kind: SqlErrorKind, msg: &str) -> Result<T> {
        self.ok_or_else(|| SqlError::new(kind, msg))
    }
}
