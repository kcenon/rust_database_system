//! SQLite database backend implementation
//!
//! This module provides a SQLite implementation of the Database trait.

#[cfg(feature = "sqlite")]
use crate::core::{
    database::Database, database_types::DatabaseType, error::DatabaseError, error::Result,
    value::DatabaseResult, value::DatabaseRow, value::DatabaseValue,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "sqlite")]
use rusqlite::{params_from_iter, Connection, Row};
use std::time::Duration;

/// Default timeout for database operations (30 seconds)
const DEFAULT_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// SQLite database implementation
#[cfg(feature = "sqlite")]
pub struct SqliteDatabase {
    connection: Arc<Mutex<Option<Connection>>>,
    in_transaction: Arc<Mutex<bool>>,
}

#[cfg(feature = "sqlite")]
impl SqliteDatabase {
    /// Create a new SQLite database instance
    pub fn new() -> Self {
        Self {
            connection: Arc::new(Mutex::new(None)),
            in_transaction: Arc::new(Mutex::new(false)),
        }
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
}

#[cfg(feature = "sqlite")]
impl Default for SqliteDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl Database for SqliteDatabase {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::Sqlite
    }

    async fn connect(&self, connection_string: &str) -> Result<()> {
        // Clean up any existing connection first
        {
            let mut connection = self.connection.lock().await;
            *connection = None;
        }

        // Reset transaction flag to handle failed/aborted attempts
        {
            let mut in_transaction = self.in_transaction.lock().await;
            *in_transaction = false;
        }

        let connection_string = connection_string.to_string();
        let connection_arc = Arc::clone(&self.connection);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = Connection::open(&connection_string)?;

            // Enable foreign keys
            conn.execute("PRAGMA foreign_keys = ON", [])?;

            let mut connection = connection_arc.blocking_lock();
            *connection = Some(conn);

            Ok(())
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))??
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::connection_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connection
            .try_lock()
            .map(|conn| conn.is_some())
            .unwrap_or(false)
    }

    async fn disconnect(&self) -> Result<()> {
        // Clear transaction flag to prevent stale state after reconnect
        {
            let mut in_transaction = self.in_transaction.lock().await;
            *in_transaction = false;
        }

        let mut connection = self.connection.lock().await;
        *connection = None;
        Ok(())
    }

    async fn execute(&self, query: &str) -> Result<u64> {
        let query = query.to_string();
        let connection_arc = Arc::clone(&self.connection);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<u64> {
            let connection = connection_arc.blocking_lock();
            let conn = connection
                .as_ref()
                .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

            let affected = conn.execute(&query, [])?;
            Ok(affected as u64)
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))?
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }
    }

    async fn query(&self, query: &str) -> Result<DatabaseResult> {
        let query = query.to_string();
        let connection_arc = Arc::clone(&self.connection);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<DatabaseResult> {
            let connection = connection_arc.blocking_lock();
            let conn = connection
                .as_ref()
                .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map([], Self::row_to_database_row)?;

            let mut results = Vec::new();
            for row_result in rows {
                results.push(row_result?);
            }

            Ok(results)
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))?
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }
    }

    async fn query_with_params(
        &self,
        query: &str,
        params: &[DatabaseValue],
    ) -> Result<DatabaseResult> {
        let query = query.to_string();
        let params = params.to_vec();
        let connection_arc = Arc::clone(&self.connection);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<DatabaseResult> {
            let connection = connection_arc.blocking_lock();
            let conn = connection
                .as_ref()
                .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

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

            Ok(results)
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))?
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }
    }

    async fn execute_with_params(&self, query: &str, params: &[DatabaseValue]) -> Result<u64> {
        let query = query.to_string();
        let params = params.to_vec();
        let connection_arc = Arc::clone(&self.connection);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<u64> {
            let connection = connection_arc.blocking_lock();
            let conn = connection
                .as_ref()
                .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

            // Convert DatabaseValue to rusqlite parameters
            let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> =
                params.iter().map(Self::value_to_param).collect();

            let mut stmt = conn.prepare(&query)?;
            let affected = stmt.execute(params_from_iter(rusqlite_params.iter()))?;

            Ok(affected as u64)
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))?
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }
    }

    async fn begin_transaction(&self) -> Result<()> {
        let connection_arc = Arc::clone(&self.connection);
        let in_transaction_arc = Arc::clone(&self.in_transaction);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<()> {
            // Acquire both locks atomically to prevent race conditions
            let mut in_transaction = in_transaction_arc.blocking_lock();
            let connection = connection_arc.blocking_lock();

            let conn = connection
                .as_ref()
                .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

            // Check if already in transaction
            if *in_transaction {
                return Err(DatabaseError::transaction(
                    "Already in a transaction".to_string(),
                ));
            }

            // Execute SQL first, only set flag on success
            conn.execute("BEGIN TRANSACTION", [])?;
            *in_transaction = true;

            Ok(())
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))?
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }
    }

    async fn commit(&self) -> Result<()> {
        let connection_arc = Arc::clone(&self.connection);
        let in_transaction_arc = Arc::clone(&self.in_transaction);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<()> {
            // Acquire both locks atomically to prevent race conditions
            let mut in_transaction = in_transaction_arc.blocking_lock();
            let connection = connection_arc.blocking_lock();

            let conn = connection
                .as_ref()
                .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

            // Check if in transaction
            if !*in_transaction {
                return Err(DatabaseError::transaction(
                    "Not in a transaction".to_string(),
                ));
            }

            // Execute SQL first, only clear flag on success
            conn.execute("COMMIT", [])?;
            *in_transaction = false;

            Ok(())
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))?
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }
    }

    async fn rollback(&self) -> Result<()> {
        let connection_arc = Arc::clone(&self.connection);
        let in_transaction_arc = Arc::clone(&self.in_transaction);

        // Offload blocking database operations to blocking thread pool with timeout
        let mut task = tokio::task::spawn_blocking(move || -> Result<()> {
            // Acquire both locks atomically to prevent race conditions
            let mut in_transaction = in_transaction_arc.blocking_lock();
            let connection = connection_arc.blocking_lock();

            let conn = connection
                .as_ref()
                .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

            // Check if in transaction
            if !*in_transaction {
                return Err(DatabaseError::transaction(
                    "Not in a transaction".to_string(),
                ));
            }

            // Execute SQL first, only clear flag on success
            conn.execute("ROLLBACK", [])?;
            *in_transaction = false;

            Ok(())
        });

        // Use select! to abort task on timeout, preventing resource leaks
        tokio::select! {
            result = &mut task => {
                result.map_err(|e| DatabaseError::other(format!("Task join error: {}", e)))?
            }
            _ = tokio::time::sleep(DEFAULT_OPERATION_TIMEOUT) => {
                task.abort();
                return Err(DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64));
            }
        }
    }

    fn in_transaction(&self) -> bool {
        self.in_transaction
            .try_lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }
}

#[cfg(feature = "sqlite")]
impl Drop for SqliteDatabase {
    fn drop(&mut self) {
        // Attempt to rollback any open transaction
        // Note: This is best-effort cleanup since Drop cannot be async
        if let Ok(in_trans) = self.in_transaction.try_lock() {
            if *in_trans {
                if let Ok(connection) = self.connection.try_lock() {
                    if let Some(conn) = connection.as_ref() {
                        let _ = conn.execute("ROLLBACK", []);
                    }
                }
            }
        }
        // Connection will be closed automatically when dropped
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_connect() {
        let db = SqliteDatabase::new();
        assert!(db.connect(":memory:").await.is_ok());
        assert!(db.is_connected());
        assert!(db.disconnect().await.is_ok());
        assert!(!db.is_connected());
    }

    #[tokio::test]
    async fn test_sqlite_execute() -> Result<()> {
        let db = SqliteDatabase::new();
        db.connect(":memory:").await?;

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
    async fn test_sqlite_query() -> Result<()> {
        let db = SqliteDatabase::new();
        db.connect(":memory:").await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await?;
        db.execute("INSERT INTO test (name) VALUES ('Alice')")
            .await?;
        db.execute("INSERT INTO test (name) VALUES ('Bob')").await?;

        let results = db.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 2);

        let name1 = results[0]
            .get("name")
            .ok_or_else(|| DatabaseError::ColumnNotFound("name".to_string()))?
            .as_string();
        assert_eq!(name1, "Alice");

        let name2 = results[1]
            .get("name")
            .ok_or_else(|| DatabaseError::ColumnNotFound("name".to_string()))?
            .as_string();
        assert_eq!(name2, "Bob");

        Ok(())
    }

    #[tokio::test]
    async fn test_sqlite_transaction() -> Result<()> {
        let db = SqliteDatabase::new();
        db.connect(":memory:").await?;

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await?;

        // Test commit
        db.begin_transaction().await?;
        assert!(db.in_transaction());

        db.execute("INSERT INTO test (name) VALUES ('Alice')")
            .await?;
        db.commit().await?;
        assert!(!db.in_transaction());

        let results = db.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 1);

        // Test rollback
        db.begin_transaction().await?;
        db.execute("INSERT INTO test (name) VALUES ('Bob')").await?;
        db.rollback().await?;
        assert!(!db.in_transaction());

        let results = db.query("SELECT * FROM test").await?;
        assert_eq!(results.len(), 1); // Still only Alice

        Ok(())
    }
}
