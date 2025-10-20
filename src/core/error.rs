//! Error types for the database system
//!
//! This module defines all error types that can occur during database operations.

/// Result type alias for database operations
pub type Result<T> = std::result::Result<T, DatabaseError>;

/// Error types for database operations
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Connection failed with details
    #[error("Connection failed to {host}:{port} - {message}")]
    ConnectionFailed {
        host: String,
        port: u16,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Connection error (generic)
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Connection timeout
    #[error("Connection timeout after {timeout_ms}ms")]
    ConnectionTimeout { timeout_ms: u64 },

    /// Connection pool exhausted
    #[error("Connection pool exhausted: {active}/{max} connections in use")]
    PoolExhausted { active: usize, max: usize },

    /// Query execution error
    #[error("Query execution error: {0}")]
    QueryError(String),

    /// Query timeout
    #[error("Query timeout after {timeout_ms}ms")]
    QueryTimeout { timeout_ms: u64 },

    /// Type conversion error
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// Invalid connection string
    #[error("Invalid connection string: {0}")]
    InvalidConnectionString(String),

    /// Database not found
    #[error("Database not found: {0}")]
    DatabaseNotFound(String),

    /// Table not found
    #[error("Table not found: {0}")]
    TableNotFound(String),

    /// Column not found
    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    TransactionError(String),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// SQLite error
    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    /// PostgreSQL error
    #[cfg(feature = "postgres")]
    #[error("PostgreSQL error: {0}")]
    PostgresError(#[from] tokio_postgres::Error),

    /// MySQL error
    #[cfg(feature = "mysql")]
    #[error("MySQL error: {0}")]
    MysqlError(String),

    /// Redis error
    #[cfg(feature = "redis_support")]
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    /// MongoDB error
    #[cfg(feature = "mongodb_support")]
    #[error("MongoDB error: {0}")]
    MongodbError(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl DatabaseError {
    /// Create a connection failed error with host/port details
    pub fn connection_failed(
        host: impl Into<String>,
        port: u16,
        message: impl Into<String>,
    ) -> Self {
        DatabaseError::ConnectionFailed {
            host: host.into(),
            port,
            message: message.into(),
            source: None,
        }
    }

    /// Create a connection failed error with source error
    pub fn connection_failed_with_source(
        host: impl Into<String>,
        port: u16,
        message: impl Into<String>,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        DatabaseError::ConnectionFailed {
            host: host.into(),
            port,
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a new connection error (generic)
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        DatabaseError::ConnectionError(msg.into())
    }

    /// Create a connection timeout error
    pub fn connection_timeout(timeout_ms: u64) -> Self {
        DatabaseError::ConnectionTimeout { timeout_ms }
    }

    /// Create a pool exhausted error
    pub fn pool_exhausted(active: usize, max: usize) -> Self {
        DatabaseError::PoolExhausted { active, max }
    }

    /// Create a new query error
    pub fn query<S: Into<String>>(msg: S) -> Self {
        DatabaseError::QueryError(msg.into())
    }

    /// Create a query timeout error
    pub fn query_timeout(timeout_ms: u64) -> Self {
        DatabaseError::QueryTimeout { timeout_ms }
    }

    /// Create a new type mismatch error
    pub fn type_mismatch(expected: &str, actual: &str) -> Self {
        DatabaseError::TypeMismatch {
            expected: expected.to_string(),
            actual: actual.to_string(),
        }
    }

    /// Create a new transaction error
    pub fn transaction<S: Into<String>>(msg: S) -> Self {
        DatabaseError::TransactionError(msg.into())
    }

    /// Create a new migration error
    pub fn migration<S: Into<String>>(msg: S) -> Self {
        DatabaseError::Migration(msg.into())
    }

    /// Create a new unsupported operation error
    pub fn unsupported<S: Into<String>>(msg: S) -> Self {
        DatabaseError::UnsupportedOperation(msg.into())
    }

    /// Create a generic error
    pub fn other<S: Into<String>>(msg: S) -> Self {
        DatabaseError::Other(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = DatabaseError::connection("Failed to connect");
        assert!(matches!(err, DatabaseError::ConnectionError(_)));

        let err = DatabaseError::query("Invalid SQL");
        assert!(matches!(err, DatabaseError::QueryError(_)));

        let err = DatabaseError::type_mismatch("i32", "String");
        assert!(matches!(err, DatabaseError::TypeMismatch { .. }));
    }

    #[test]
    fn test_error_display() {
        let err = DatabaseError::connection("Connection refused");
        assert_eq!(err.to_string(), "Connection error: Connection refused");

        let err = DatabaseError::type_mismatch("i64", "f64");
        assert_eq!(err.to_string(), "Type mismatch: expected i64, got f64");
    }
}
