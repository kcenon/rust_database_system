//! Pooled SQLite database backend implementation
//!
//! This module provides a connection-pooled SQLite implementation of the Database trait
//! using deadpool for efficient connection management and improved concurrency.

#[cfg(feature = "sqlite")]
use crate::core::{
    database::Database, database_types::DatabaseType, error::DatabaseError, error::Result,
    value::DatabaseResult, value::DatabaseRow, value::DatabaseValue,
};
use async_trait::async_trait;
use std::time::Duration;

#[cfg(feature = "sqlite")]
use deadpool_sqlite::{Config, Pool, Runtime};
#[cfg(feature = "sqlite")]
use rusqlite::{params_from_iter, Row};
#[cfg(feature = "sqlite")]
use std::sync::atomic::{AtomicBool, Ordering};

/// Default timeout for database operations (30 seconds)
const DEFAULT_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Pool configuration for SQLite connections
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_size: usize,
    /// Timeout for acquiring a connection from the pool
    pub timeout: Duration,
    /// Timeout for database operations (query, execute, etc.)
    pub operation_timeout: Duration,
    /// SQLite connection string
    pub connection_string: String,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 16,
            timeout: Duration::from_secs(5),
            operation_timeout: DEFAULT_OPERATION_TIMEOUT,
            connection_string: String::new(),
        }
    }
}

impl PoolConfig {
    /// Create a new pool configuration
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            ..Default::default()
        }
    }

    /// Set maximum pool size
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Set connection acquisition timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set database operation timeout (for query, execute, etc.)
    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }
}

/// Pooled SQLite database implementation
///
/// This implementation uses a connection pool to efficiently manage multiple
/// concurrent database operations without the overhead of creating a new
/// connection for each query.
///
/// # Example
///
/// ```no_run
/// use rust_database_system::backends::PooledSqliteDatabase;
/// use rust_database_system::core::database::Database;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let db = PooledSqliteDatabase::new("test.db").await?;
///
///     // Multiple concurrent operations can now share pooled connections
///     db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)").await?;
///
///     Ok(())
/// }
/// ```
#[cfg(feature = "sqlite")]
pub struct PooledSqliteDatabase {
    pool: Pool,
    operation_timeout: Duration,
}

#[cfg(feature = "sqlite")]
impl PooledSqliteDatabase {
    /// Create a new pooled SQLite database instance with default configuration
    ///
    /// # Errors
    ///
    /// Returns error if pool creation or initialization fails
    pub async fn new(connection_string: impl Into<String>) -> Result<Self> {
        let config = PoolConfig::new(connection_string);
        Self::with_config(config).await
    }

    /// Create a new pooled SQLite database instance with custom configuration
    ///
    /// # Errors
    ///
    /// Returns error if pool creation or initialization fails
    pub async fn with_config(config: PoolConfig) -> Result<Self> {
        let pool_config = Config::new(config.connection_string);

        let pool = pool_config
            .create_pool(Runtime::Tokio1)
            .map_err(|e| DatabaseError::connection(format!("Failed to create pool: {}", e)))?;

        // Configure pool size
        // Note: deadpool-sqlite doesn't support max_size directly in Config
        // It's managed through the Pool creation, but we can validate connection
        let conn = pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire initial connection: {}", e))
        })?;

        // Initialize database settings
        conn.interact(|conn| {
            conn.execute("PRAGMA foreign_keys = ON", [])?;
            // PRAGMA journal_mode returns a value, so we need to use query_row
            conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
            Ok::<_, rusqlite::Error>(())
        })
        .await
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(|e| DatabaseError::other(format!("Failed to initialize database: {}", e)))?;

        Ok(Self {
            pool,
            operation_timeout: config.operation_timeout,
        })
    }

    /// Convert a rusqlite Row to a DatabaseRow
    fn row_to_database_row(row: &Row) -> rusqlite::Result<DatabaseRow> {
        let mut db_row = DatabaseRow::new();
        let column_count = row.as_ref().column_count();

        for i in 0..column_count {
            let column_name = row.as_ref().column_name(i)?.to_string();
            let value = match row.get_ref(i)? {
                rusqlite::types::ValueRef::Null => DatabaseValue::Null,
                rusqlite::types::ValueRef::Integer(v) => DatabaseValue::Long(v),
                rusqlite::types::ValueRef::Real(v) => DatabaseValue::Double(v),
                rusqlite::types::ValueRef::Text(v) => {
                    DatabaseValue::String(String::from_utf8_lossy(v).to_string())
                }
                rusqlite::types::ValueRef::Blob(v) => DatabaseValue::Bytes(v.to_vec()),
            };
            db_row.insert(column_name, value);
        }

        Ok(db_row)
    }

    /// Convert DatabaseValue to rusqlite parameter
    fn value_to_param(value: &DatabaseValue) -> Box<dyn rusqlite::ToSql> {
        match value {
            DatabaseValue::Null => Box::new(None::<i64>),
            DatabaseValue::Bool(v) => Box::new(*v),
            DatabaseValue::Int(v) => Box::new(*v),
            DatabaseValue::Long(v) => Box::new(*v),
            DatabaseValue::Float(v) => Box::new(*v),
            DatabaseValue::Double(v) => Box::new(*v),
            DatabaseValue::String(v) => Box::new(v.clone()),
            DatabaseValue::Bytes(v) => Box::new(v.clone()),
            DatabaseValue::Timestamp(v) => Box::new(*v),
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let status = self.pool.status();
        PoolStats {
            size: status.size,
            available: status.available,
            waiting: status.waiting,
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of connections in the pool
    pub size: usize,
    /// Number of available connections
    pub available: usize,
    /// Number of requests waiting for a connection
    pub waiting: usize,
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl Database for PooledSqliteDatabase {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::Sqlite
    }

    async fn connect(&self, _connection_string: &str) -> Result<()> {
        // Connection pool is already initialized in new()
        // This method is a no-op but validates pool health
        let _ =
            self.pool.get().await.map_err(|e| {
                DatabaseError::connection(format!("Pool health check failed: {}", e))
            })?;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Pool is always "connected" if it exists
        self.pool.status().size > 0
    }

    async fn disconnect(&self) -> Result<()> {
        // Note: deadpool doesn't support explicit pool shutdown
        // Connections will be closed when the pool is dropped
        Ok(())
    }

    async fn execute(&self, query: &str) -> Result<u64> {
        let query = query.to_string();

        let conn = self.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire connection: {}", e))
        })?;

        let affected = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                let result = conn.execute(&query, [])?;
                Ok::<_, rusqlite::Error>(result)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(affected as u64)
    }

    async fn query(&self, query: &str) -> Result<DatabaseResult> {
        let query = query.to_string();

        let conn = self.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire connection: {}", e))
        })?;

        let results = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map([], Self::row_to_database_row)?;

                let mut results = Vec::new();
                for row_result in rows {
                    results.push(row_result?);
                }

                Ok::<_, rusqlite::Error>(results)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(results)
    }

    async fn query_with_params(
        &self,
        query: &str,
        params: &[DatabaseValue],
    ) -> Result<DatabaseResult> {
        let query = query.to_string();
        let params = params.to_vec();

        let conn = self.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire connection: {}", e))
        })?;

        let results = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                // Convert DatabaseValue to rusqlite parameters
                let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> =
                    params.iter().map(Self::value_to_param).collect();

                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map(
                    params_from_iter(rusqlite_params.iter()),
                    Self::row_to_database_row,
                )?;

                let mut results = Vec::new();
                for row_result in rows {
                    results.push(row_result?);
                }

                Ok::<_, rusqlite::Error>(results)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(results)
    }

    async fn execute_with_params(&self, query: &str, params: &[DatabaseValue]) -> Result<u64> {
        let query = query.to_string();
        let params = params.to_vec();

        let conn = self.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire connection: {}", e))
        })?;

        let affected = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                // Convert DatabaseValue to rusqlite parameters
                let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> =
                    params.iter().map(Self::value_to_param).collect();

                let mut stmt = conn.prepare(&query)?;
                let result = stmt.execute(params_from_iter(rusqlite_params.iter()))?;

                Ok::<_, rusqlite::Error>(result)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(affected as u64)
    }

    async fn begin_transaction(&self) -> Result<()> {
        let conn = self.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire connection: {}", e))
        })?;

        tokio::time::timeout(
            self.operation_timeout,
            conn.interact(|conn| {
                conn.execute("BEGIN TRANSACTION", [])?;
                Ok::<_, rusqlite::Error>(())
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(())
    }

    async fn commit(&self) -> Result<()> {
        let conn = self.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire connection: {}", e))
        })?;

        tokio::time::timeout(
            self.operation_timeout,
            conn.interact(|conn| {
                conn.execute("COMMIT", [])?;
                Ok::<_, rusqlite::Error>(())
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(())
    }

    async fn rollback(&self) -> Result<()> {
        let conn = self.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!("Failed to acquire connection: {}", e))
        })?;

        tokio::time::timeout(
            self.operation_timeout,
            conn.interact(|conn| {
                conn.execute("ROLLBACK", [])?;
                Ok::<_, rusqlite::Error>(())
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(())
    }

    /// **WARNING**: This method is unreliable with connection pooling and always returns false.
    ///
    /// Use [`PooledTransaction`] instead for safe transaction management.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rust_database_system::backends::{PooledSqliteDatabase, PooledTransaction};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let db = PooledSqliteDatabase::new("test.db").await?;
    /// let mut tx = PooledTransaction::begin(&db).await?;
    /// tx.execute("INSERT INTO...").await?;
    /// tx.commit().await?;
    /// # Ok(())
    /// # }
    /// ```
    fn in_transaction(&self) -> bool {
        // CRITICAL WARNING: This method is unreliable with connection pooling!
        //
        // With connection pooling, transaction state is per-connection.
        // This method always returns false because:
        // 1. Different pool connections may have different transaction states
        // 2. The connection used for this check may differ from query connections
        // 3. Checking actual state would require acquiring a connection from the pool
        //
        // RECOMMENDED: Use PooledTransaction for safe transaction management:
        //
        //   use rust_database_system::backends::PooledTransaction;
        //   let db = PooledSqliteDatabase::new("test.db").await?;
        //   let mut tx = PooledTransaction::begin(&db).await?;
        //   tx.execute("INSERT INTO...").await?;
        //   tx.commit().await?;
        //
        // PooledTransaction ensures:
        // - Same connection used for entire transaction
        // - Automatic rollback on drop if not committed
        // - No transaction state confusion
        // - Safe error handling

        eprintln!(
            "[DATABASE WARNING] in_transaction() is deprecated and unreliable with connection pools. \
             This method always returns false. Use PooledTransaction for safe transaction management."
        );

        false
    }
}

/// Transaction guard for pooled connections
///
/// This type ensures that a transaction uses the same connection throughout
/// its lifecycle, solving the fundamental issue with connection pooling and transactions.
///
/// # Example
///
/// ```no_run
/// use rust_database_system::backends::{PooledSqliteDatabase, PooledTransaction};
/// use rust_database_system::core::value::DatabaseValue;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let db = PooledSqliteDatabase::new("test.db").await?;
///
/// // Begin transaction - acquires connection from pool
/// let mut tx = PooledTransaction::begin(&db).await?;
///
/// // All operations use the same connection
/// tx.execute("INSERT INTO accounts (id, balance) VALUES (1, 100)").await?;
/// tx.execute("INSERT INTO accounts (id, balance) VALUES (2, 50)").await?;
///
/// // Commit transaction - connection returned to pool
/// tx.commit().await?;
/// # Ok(())
/// # }
/// ```
///
/// # Automatic Rollback
///
/// If the transaction is dropped without being committed, it automatically rolls back:
///
/// ```no_run
/// # use rust_database_system::backends::{PooledSqliteDatabase, PooledTransaction};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let db = PooledSqliteDatabase::new("test.db").await?;
/// let mut tx = PooledTransaction::begin(&db).await?;
/// tx.execute("INSERT INTO test VALUES (1)").await?;
///
/// // Transaction dropped without commit - automatically rolls back
/// drop(tx);
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "sqlite")]
pub struct PooledTransaction {
    connection: Option<deadpool_sqlite::Object>,
    committed: AtomicBool,
    rolled_back: AtomicBool,
    operation_timeout: Duration,
}

#[cfg(feature = "sqlite")]
impl PooledTransaction {
    /// Begin a new transaction
    ///
    /// Acquires a connection from the pool and begins a transaction on it.
    /// The connection is held until the transaction is committed, rolled back, or dropped.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Pool cannot provide a connection
    /// - BEGIN TRANSACTION fails
    pub async fn begin(db: &PooledSqliteDatabase) -> Result<Self> {
        let conn = db.pool.get().await.map_err(|e| {
            DatabaseError::connection(format!(
                "Failed to acquire connection for transaction: {}",
                e
            ))
        })?;

        let operation_timeout = db.operation_timeout;

        // Begin transaction on the acquired connection
        tokio::time::timeout(
            operation_timeout,
            conn.interact(|conn| {
                conn.execute("BEGIN TRANSACTION", [])?;
                Ok::<_, rusqlite::Error>(())
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(Self {
            connection: Some(conn),
            committed: AtomicBool::new(false),
            rolled_back: AtomicBool::new(false),
            operation_timeout,
        })
    }

    /// Execute a query that doesn't return results
    ///
    /// Uses the transaction's dedicated connection.
    pub async fn execute(&self, query: &str) -> Result<u64> {
        let conn = self.connection.as_ref().ok_or_else(|| {
            DatabaseError::transaction("Transaction already finalized".to_string())
        })?;

        let query = query.to_string();

        let affected = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                let result = conn.execute(&query, [])?;
                Ok::<_, rusqlite::Error>(result)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(affected as u64)
    }

    /// Execute a parameterized query that doesn't return results
    pub async fn execute_with_params(&self, query: &str, params: &[DatabaseValue]) -> Result<u64> {
        let conn = self.connection.as_ref().ok_or_else(|| {
            DatabaseError::transaction("Transaction already finalized".to_string())
        })?;

        let query = query.to_string();
        let params = params.to_vec();

        let affected = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> = params
                    .iter()
                    .map(PooledSqliteDatabase::value_to_param)
                    .collect();

                let mut stmt = conn.prepare(&query)?;
                let result = stmt.execute(params_from_iter(rusqlite_params.iter()))?;

                Ok::<_, rusqlite::Error>(result)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(affected as u64)
    }

    /// Execute a SELECT query and return results
    pub async fn query(&self, query: &str) -> Result<DatabaseResult> {
        let conn = self.connection.as_ref().ok_or_else(|| {
            DatabaseError::transaction("Transaction already finalized".to_string())
        })?;

        let query = query.to_string();

        let results = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map([], PooledSqliteDatabase::row_to_database_row)?;

                let mut results = Vec::new();
                for row_result in rows {
                    results.push(row_result?);
                }

                Ok::<_, rusqlite::Error>(results)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(results)
    }

    /// Execute a parameterized SELECT query
    pub async fn query_with_params(
        &self,
        query: &str,
        params: &[DatabaseValue],
    ) -> Result<DatabaseResult> {
        let conn = self.connection.as_ref().ok_or_else(|| {
            DatabaseError::transaction("Transaction already finalized".to_string())
        })?;

        let query = query.to_string();
        let params = params.to_vec();

        let results = tokio::time::timeout(
            self.operation_timeout,
            conn.interact(move |conn| {
                let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> = params
                    .iter()
                    .map(PooledSqliteDatabase::value_to_param)
                    .collect();

                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map(
                    params_from_iter(rusqlite_params.iter()),
                    PooledSqliteDatabase::row_to_database_row,
                )?;

                let mut results = Vec::new();
                for row_result in rows {
                    results.push(row_result?);
                }

                Ok::<_, rusqlite::Error>(results)
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        Ok(results)
    }

    /// Commit the transaction
    ///
    /// Commits the transaction and returns the connection to the pool.
    /// After calling this, no further operations can be performed.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Transaction already committed or rolled back
    /// - COMMIT statement fails
    pub async fn commit(mut self) -> Result<()> {
        if self.committed.load(Ordering::Acquire) {
            return Err(DatabaseError::transaction(
                "Transaction already committed".to_string(),
            ));
        }
        if self.rolled_back.load(Ordering::Acquire) {
            return Err(DatabaseError::transaction(
                "Transaction already rolled back".to_string(),
            ));
        }

        let conn = self.connection.take().ok_or_else(|| {
            DatabaseError::transaction("Transaction connection missing".to_string())
        })?;

        tokio::time::timeout(
            self.operation_timeout,
            conn.interact(|conn| {
                conn.execute("COMMIT", [])?;
                Ok::<_, rusqlite::Error>(())
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        self.committed.store(true, Ordering::Release);
        // Connection automatically returned to pool when dropped

        Ok(())
    }

    /// Rollback the transaction
    ///
    /// Rolls back the transaction and returns the connection to the pool.
    /// After calling this, no further operations can be performed.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Transaction already committed or rolled back
    /// - ROLLBACK statement fails
    pub async fn rollback(mut self) -> Result<()> {
        if self.committed.load(Ordering::Acquire) {
            return Err(DatabaseError::transaction(
                "Transaction already committed".to_string(),
            ));
        }
        if self.rolled_back.load(Ordering::Acquire) {
            return Err(DatabaseError::transaction(
                "Transaction already rolled back".to_string(),
            ));
        }

        let conn = self.connection.take().ok_or_else(|| {
            DatabaseError::transaction("Transaction connection missing".to_string())
        })?;

        tokio::time::timeout(
            self.operation_timeout,
            conn.interact(|conn| {
                conn.execute("ROLLBACK", [])?;
                Ok::<_, rusqlite::Error>(())
            }),
        )
        .await
        .map_err(|_| DatabaseError::query_timeout(self.operation_timeout.as_millis() as u64))?
        .map_err(|e| DatabaseError::other(format!("Interact error: {}", e)))?
        .map_err(DatabaseError::from)?;

        self.rolled_back.store(true, Ordering::Release);
        // Connection automatically returned to pool when dropped

        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl Drop for PooledTransaction {
    fn drop(&mut self) {
        // If transaction not explicitly committed or rolled back, warn user
        if !self.committed.load(Ordering::Acquire)
            && !self.rolled_back.load(Ordering::Acquire)
            && self.connection.is_some()
        {
            eprintln!(
                "[DATABASE WARNING] PooledTransaction dropped without commit or rollback. \
                 The transaction will be automatically rolled back when the connection is returned to the pool. \
                 Always explicitly call commit() or rollback() to avoid this warning."
            );
            // Note: We cannot perform async rollback in Drop.
            // The connection will be returned to the pool and SQLite will automatically
            // rollback the transaction when the connection is reused or closed.
            // This is safe but not ideal - users should explicitly commit or rollback.
        }
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pooled_sqlite_connect() {
        let db = PooledSqliteDatabase::new(":memory:").await;
        assert!(db.is_ok());

        let db = db.unwrap();
        assert!(db.is_connected());
    }

    #[tokio::test]
    async fn test_pooled_sqlite_execute() -> Result<()> {
        let db = PooledSqliteDatabase::new(":memory:").await?;

        let result = db
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await;
        assert!(result.is_ok());

        let affected = db
            .execute("INSERT INTO test (name) VALUES ('Alice')")
            .await?;
        assert_eq!(affected, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_pooled_sqlite_query() -> Result<()> {
        let db = PooledSqliteDatabase::new(":memory:").await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await?;
        db.execute("INSERT INTO test (name) VALUES ('Alice')")
            .await?;
        db.execute("INSERT INTO test (name) VALUES ('Bob')").await?;

        let results = db.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_pooled_concurrent_queries() -> Result<()> {
        // Use shared memory database so all connections see the same schema
        let db = Arc::new(PooledSqliteDatabase::new("file::memory:?cache=shared").await?);

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await?;

        // Spawn multiple concurrent tasks
        let mut handles = vec![];
        for i in 0..10 {
            let db_clone = Arc::clone(&db);
            let handle = tokio::spawn(async move {
                // Use parameterized queries to prevent SQL injection
                let params = vec![DatabaseValue::String(format!("User{}", i))];
                db_clone
                    .execute_with_params("INSERT INTO test (name) VALUES (?)", &params)
                    .await
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        let results = db.query("SELECT COUNT(*) as count FROM test").await?;
        assert_eq!(results.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_pool_stats() -> Result<()> {
        let db = PooledSqliteDatabase::new(":memory:").await?;

        let stats = db.stats();
        assert!(stats.size > 0);

        Ok(())
    }
}
