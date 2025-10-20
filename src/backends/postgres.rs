//! PostgreSQL database backend implementation
//!
//! This module provides a PostgreSQL implementation of the Database trait using tokio-postgres.

use crate::core::{
    database::Database, database_types::DatabaseType, error::DatabaseError, error::Result,
    value::DatabaseResult, value::DatabaseRow, value::DatabaseValue,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_postgres::{Client, NoTls, Row};

/// Default timeout for database operations (30 seconds)
const DEFAULT_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// PostgreSQL database implementation
pub struct PostgresDatabase {
    client: Arc<Mutex<Option<Client>>>,
    in_transaction: Arc<Mutex<bool>>,
}

impl PostgresDatabase {
    /// Create a new PostgreSQL database instance
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            in_transaction: Arc::new(Mutex::new(false)),
        }
    }

    /// Convert a tokio_postgres Row to a DatabaseRow
    fn row_to_database_row(row: &Row) -> DatabaseRow {
        let mut db_row = DatabaseRow::new();

        for (idx, column) in row.columns().iter().enumerate() {
            let column_name = column.name().to_string();
            let value = match column.type_().name() {
                "bool" => row
                    .get::<_, Option<bool>>(idx)
                    .map(DatabaseValue::Bool)
                    .unwrap_or(DatabaseValue::Null),
                "int2" | "int4" => row
                    .get::<_, Option<i32>>(idx)
                    .map(DatabaseValue::Int)
                    .unwrap_or(DatabaseValue::Null),
                "int8" => row
                    .get::<_, Option<i64>>(idx)
                    .map(DatabaseValue::Long)
                    .unwrap_or(DatabaseValue::Null),
                "float4" => row
                    .get::<_, Option<f32>>(idx)
                    .map(DatabaseValue::Float)
                    .unwrap_or(DatabaseValue::Null),
                "float8" => row
                    .get::<_, Option<f64>>(idx)
                    .map(DatabaseValue::Double)
                    .unwrap_or(DatabaseValue::Null),
                "text" | "varchar" | "char" | "bpchar" => row
                    .get::<_, Option<String>>(idx)
                    .map(DatabaseValue::String)
                    .unwrap_or(DatabaseValue::Null),
                "bytea" => row
                    .get::<_, Option<Vec<u8>>>(idx)
                    .map(DatabaseValue::Bytes)
                    .unwrap_or(DatabaseValue::Null),
                "timestamp" | "timestamptz" => row
                    .get::<_, Option<i64>>(idx)
                    .map(DatabaseValue::Timestamp)
                    .unwrap_or(DatabaseValue::Null),
                _ => {
                    // Try to get as string for unknown types
                    row.get::<_, Option<String>>(idx)
                        .map(DatabaseValue::String)
                        .unwrap_or(DatabaseValue::Null)
                }
            };
            db_row.insert(column_name, value);
        }

        db_row
    }

    /// Convert DatabaseValue to postgres parameter
    fn value_to_param(
        value: &DatabaseValue,
    ) -> Box<dyn tokio_postgres::types::ToSql + Sync + Send> {
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

impl Default for PostgresDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Database for PostgresDatabase {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::Postgres
    }

    async fn connect(&self, connection_string: &str) -> Result<()> {
        // Clean up any existing connection first
        {
            let mut client = self.client.lock().await;
            *client = None;
        }

        // Reset transaction flag
        {
            let mut in_transaction = self.in_transaction.lock().await;
            *in_transaction = false;
        }

        let connection_string = connection_string.to_string();
        let client_arc = Arc::clone(&self.client);

        // Connect with timeout
        let connect_future = async move {
            let (client, connection) = tokio_postgres::connect(&connection_string, NoTls)
                .await
                .map_err(|e| DatabaseError::connection(e.to_string()))?;

            // Spawn the connection handler in the background
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("PostgreSQL connection error: {}", e);
                }
            });

            let mut client_guard = client_arc.lock().await;
            *client_guard = Some(client);

            Ok::<(), DatabaseError>(())
        };

        tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, connect_future)
            .await
            .map_err(|_| {
                DatabaseError::connection_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })??;

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.client
            .try_lock()
            .map(|client| {
                if let Some(ref c) = *client {
                    !c.is_closed()
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    async fn disconnect(&self) -> Result<()> {
        // Clear transaction flag
        {
            let mut in_transaction = self.in_transaction.lock().await;
            *in_transaction = false;
        }

        let mut client = self.client.lock().await;
        *client = None;
        Ok(())
    }

    async fn execute(&self, query: &str) -> Result<u64> {
        let client = self.client.lock().await;
        let client = client
            .as_ref()
            .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

        let execute_future = client.execute(query, &[]);

        let affected = tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, execute_future)
            .await
            .map_err(|_| {
                DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })?
            .map_err(|e| DatabaseError::query(e.to_string()))?;

        Ok(affected)
    }

    async fn query(&self, query: &str) -> Result<DatabaseResult> {
        let client = self.client.lock().await;
        let client = client
            .as_ref()
            .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

        let query_future = client.query(query, &[]);

        let rows = tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, query_future)
            .await
            .map_err(|_| {
                DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })?
            .map_err(|e| DatabaseError::query(e.to_string()))?;

        let results: Vec<DatabaseRow> = rows.iter().map(Self::row_to_database_row).collect();

        Ok(results)
    }

    async fn query_with_params(
        &self,
        query: &str,
        params: &[DatabaseValue],
    ) -> Result<DatabaseResult> {
        let client = self.client.lock().await;
        let client = client
            .as_ref()
            .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

        // Convert DatabaseValue to postgres parameters and extract references
        let postgres_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
            params.iter().map(Self::value_to_param).collect();

        // Create a slice of trait object references
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = postgres_params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let query_future = client.query(query, &param_refs);

        let rows = tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, query_future)
            .await
            .map_err(|_| {
                DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })?
            .map_err(|e| DatabaseError::query(e.to_string()))?;

        let results: Vec<DatabaseRow> = rows.iter().map(Self::row_to_database_row).collect();

        Ok(results)
    }

    async fn execute_with_params(&self, query: &str, params: &[DatabaseValue]) -> Result<u64> {
        let client = self.client.lock().await;
        let client = client
            .as_ref()
            .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

        // Convert DatabaseValue to postgres parameters and extract references
        let postgres_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
            params.iter().map(Self::value_to_param).collect();

        // Create a slice of trait object references
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = postgres_params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let execute_future = client.execute(query, &param_refs);

        let affected = tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, execute_future)
            .await
            .map_err(|_| {
                DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })?
            .map_err(|e| DatabaseError::query(e.to_string()))?;

        Ok(affected)
    }

    async fn begin_transaction(&self) -> Result<()> {
        let mut in_transaction = self.in_transaction.lock().await;

        if *in_transaction {
            return Err(DatabaseError::transaction(
                "Already in a transaction".to_string(),
            ));
        }

        let client = self.client.lock().await;
        let client = client
            .as_ref()
            .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

        let begin_future = client.execute("BEGIN", &[]);

        tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, begin_future)
            .await
            .map_err(|_| {
                DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })?
            .map_err(|e| DatabaseError::transaction(e.to_string()))?;

        *in_transaction = true;

        Ok(())
    }

    async fn commit(&self) -> Result<()> {
        let mut in_transaction = self.in_transaction.lock().await;

        if !*in_transaction {
            return Err(DatabaseError::transaction(
                "Not in a transaction".to_string(),
            ));
        }

        let client = self.client.lock().await;
        let client = client
            .as_ref()
            .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

        let commit_future = client.execute("COMMIT", &[]);

        tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, commit_future)
            .await
            .map_err(|_| {
                DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })?
            .map_err(|e| DatabaseError::transaction(e.to_string()))?;

        *in_transaction = false;

        Ok(())
    }

    async fn rollback(&self) -> Result<()> {
        let mut in_transaction = self.in_transaction.lock().await;

        if !*in_transaction {
            return Err(DatabaseError::transaction(
                "Not in a transaction".to_string(),
            ));
        }

        let client = self.client.lock().await;
        let client = client
            .as_ref()
            .ok_or_else(|| DatabaseError::connection("Not connected to database"))?;

        let rollback_future = client.execute("ROLLBACK", &[]);

        tokio::time::timeout(DEFAULT_OPERATION_TIMEOUT, rollback_future)
            .await
            .map_err(|_| {
                DatabaseError::query_timeout(DEFAULT_OPERATION_TIMEOUT.as_millis() as u64)
            })?
            .map_err(|e| DatabaseError::transaction(e.to_string()))?;

        *in_transaction = false;

        Ok(())
    }

    fn in_transaction(&self) -> bool {
        self.in_transaction
            .try_lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }
}

impl Drop for PostgresDatabase {
    fn drop(&mut self) {
        // Attempt to rollback any open transaction
        // Note: This is best-effort cleanup since Drop cannot be async
        if let Ok(in_trans) = self.in_transaction.try_lock() {
            if *in_trans {
                // Connection will handle cleanup when dropped
                // We can't execute async rollback here
            }
        }
        // Client will be closed automatically when dropped
    }
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    fn get_postgres_url() -> Option<String> {
        std::env::var("POSTGRES_URL").ok()
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test --features postgres -- --ignored
    async fn test_postgres_connect() {
        let url = match get_postgres_url() {
            Some(url) => url,
            None => {
                eprintln!("Skipping test: POSTGRES_URL not set");
                return;
            }
        };

        let db = PostgresDatabase::new();
        assert!(db.connect(&url).await.is_ok());
        assert!(db.is_connected());
        assert!(db.disconnect().await.is_ok());
        assert!(!db.is_connected());
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test --features postgres -- --ignored
    async fn test_postgres_execute() -> Result<()> {
        let url = match get_postgres_url() {
            Some(url) => url,
            None => {
                eprintln!("Skipping test: POSTGRES_URL not set");
                return Ok(());
            }
        };

        let db = PostgresDatabase::new();
        db.connect(&url).await?;

        // Create a temporary table
        let _ = db.execute("DROP TABLE IF EXISTS test_execute").await;
        let result = db
            .execute("CREATE TABLE test_execute (id SERIAL PRIMARY KEY, name TEXT)")
            .await;
        assert!(result.is_ok());

        let affected = db
            .execute("INSERT INTO test_execute (name) VALUES ('Alice')")
            .await?;
        assert_eq!(affected, 1);

        // Cleanup
        db.execute("DROP TABLE test_execute").await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test --features postgres -- --ignored
    async fn test_postgres_query() -> Result<()> {
        let url = match get_postgres_url() {
            Some(url) => url,
            None => {
                eprintln!("Skipping test: POSTGRES_URL not set");
                return Ok(());
            }
        };

        let db = PostgresDatabase::new();
        db.connect(&url).await?;

        // Create and populate test table
        let _ = db.execute("DROP TABLE IF EXISTS test_query").await;
        db.execute("CREATE TABLE test_query (id SERIAL PRIMARY KEY, name TEXT)")
            .await?;
        db.execute("INSERT INTO test_query (name) VALUES ('Alice')")
            .await?;
        db.execute("INSERT INTO test_query (name) VALUES ('Bob')")
            .await?;

        let results = db.query("SELECT * FROM test_query ORDER BY id").await?;
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

        // Cleanup
        db.execute("DROP TABLE test_query").await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test --features postgres -- --ignored
    async fn test_postgres_transaction() -> Result<()> {
        let url = match get_postgres_url() {
            Some(url) => url,
            None => {
                eprintln!("Skipping test: POSTGRES_URL not set");
                return Ok(());
            }
        };

        let db = PostgresDatabase::new();
        db.connect(&url).await?;

        // Create test table
        let _ = db.execute("DROP TABLE IF EXISTS test_transaction").await;
        db.execute("CREATE TABLE test_transaction (id SERIAL PRIMARY KEY, name TEXT)")
            .await?;

        // Test commit
        db.begin_transaction().await?;
        assert!(db.in_transaction());

        db.execute("INSERT INTO test_transaction (name) VALUES ('Alice')")
            .await?;
        db.commit().await?;
        assert!(!db.in_transaction());

        let results = db.query("SELECT * FROM test_transaction").await?;
        assert_eq!(results.len(), 1);

        // Test rollback
        db.begin_transaction().await?;
        db.execute("INSERT INTO test_transaction (name) VALUES ('Bob')")
            .await?;
        db.rollback().await?;
        assert!(!db.in_transaction());

        let results = db.query("SELECT * FROM test_transaction").await?;
        assert_eq!(results.len(), 1); // Still only Alice

        // Cleanup
        db.execute("DROP TABLE test_transaction").await?;
        Ok(())
    }
}
