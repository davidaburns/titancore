use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {query}\nParams: {params:?}\nCause: {source}")]
    QueryError {
        #[source]
        source: sqlx::Error,
        query: String,
        params: Vec<String>,
    },

    #[error("Transaction error: {0}")]
    TransactionError(#[from] sqlx::Error),

    #[error("Query timeout after {0:?}")]
    Timeout(Duration),

    #[error("Panic during query execution: {0}")]
    Panic(String),

    #[error("Connection pool error: {0}")]
    PoolError(String),
}
