//! Database trait and connection management
//!
//! This module defines the core database trait that all database backends must implement.

use super::database_types::DatabaseType;
use super::error::Result;
use super::value::{DatabaseResult, DatabaseValue};
use async_trait::async_trait;

/// Core database trait that all database backends must implement
#[async_trait]
pub trait Database: Send + Sync {
    /// Get the database type
    fn database_type(&self) -> DatabaseType;

    /// Connect to the database with the given connection string
    ///
    /// # Thread Safety
    /// This method uses interior mutability via Arc<Mutex<>> so it's safe to call
    /// from multiple threads concurrently, though only one connection operation
    /// will proceed at a time.
    async fn connect(&self, connection_string: &str) -> Result<()>;

    /// Check if connected to the database
    fn is_connected(&self) -> bool;

    /// Disconnect from the database
    ///
    /// # Thread Safety
    /// This method uses interior mutability via Arc<Mutex<>> so it's safe to call
    /// from multiple threads concurrently.
    async fn disconnect(&self) -> Result<()>;

    /// Execute a query that doesn't return results (INSERT, UPDATE, DELETE, CREATE, etc.)
    ///
    /// # Security Warning
    ///
    /// **SQL Injection Risk**: This method executes raw SQL without parameter sanitization.
    /// Never use this method with user input unless the input is properly validated and sanitized.
    ///
    /// # Recommended Practice
    ///
    /// Use `execute_with_params()` instead for queries with user input:
    ///
    /// ```ignore
    /// // UNSAFE - vulnerable to SQL injection
    /// db.execute(&format!("INSERT INTO users (name) VALUES ('{}')", user_input)).await?;
    ///
    /// // SAFE - uses parameterized query
    /// db.execute_with_params(
    ///     "INSERT INTO users (name) VALUES (?)",
    ///     &[DatabaseValue::String(user_input)]
    /// ).await?;
    /// ```
    ///
    /// # Thread Safety
    /// Safe to call concurrently from multiple threads. Operations are serialized internally.
    async fn execute(&self, query: &str) -> Result<u64>;

    /// Execute a SELECT query and return results
    ///
    /// # Security Warning
    ///
    /// **SQL Injection Risk**: This method executes raw SQL without parameter sanitization.
    /// Never use this method with user input unless the input is properly validated and sanitized.
    ///
    /// # Recommended Practice
    ///
    /// Use `query_with_params()` instead for queries with user input:
    ///
    /// ```ignore
    /// // UNSAFE - vulnerable to SQL injection
    /// db.query(&format!("SELECT * FROM users WHERE name = '{}'", user_input)).await?;
    ///
    /// // SAFE - uses parameterized query
    /// db.query_with_params(
    ///     "SELECT * FROM users WHERE name = ?",
    ///     &[DatabaseValue::String(user_input)]
    /// ).await?;
    /// ```
    ///
    /// # Thread Safety
    /// Safe to call concurrently from multiple threads. Operations are serialized internally.
    async fn query(&self, query: &str) -> Result<DatabaseResult>;

    /// Execute a query with parameters (prepared statement)
    ///
    /// This method uses parameterized queries to prevent SQL injection attacks.
    /// Always use this method when executing queries with user input.
    ///
    /// # Thread Safety
    /// Safe to call concurrently from multiple threads. Operations are serialized internally.
    async fn query_with_params(
        &self,
        query: &str,
        params: &[DatabaseValue],
    ) -> Result<DatabaseResult>;

    /// Execute a query with parameters that doesn't return results (INSERT, UPDATE, DELETE, etc.)
    ///
    /// This method uses parameterized queries to prevent SQL injection attacks.
    /// Always use this method when executing queries with user input.
    ///
    /// # Thread Safety
    /// Safe to call concurrently from multiple threads. Operations are serialized internally.
    async fn execute_with_params(&self, query: &str, params: &[DatabaseValue]) -> Result<u64>;

    /// Begin a transaction
    ///
    /// # Thread Safety
    /// Safe to call concurrently. Only one transaction can be active at a time per connection.
    async fn begin_transaction(&self) -> Result<()>;

    /// Commit the current transaction
    ///
    /// # Thread Safety
    /// Safe to call concurrently, though only meaningful if a transaction is active.
    async fn commit(&self) -> Result<()>;

    /// Rollback the current transaction
    ///
    /// # Thread Safety
    /// Safe to call concurrently, though only meaningful if a transaction is active.
    async fn rollback(&self) -> Result<()>;

    /// Check if currently in a transaction
    fn in_transaction(&self) -> bool;

    /// Execute multiple queries in a transaction
    ///
    /// # Note
    /// This method has a default implementation that calls begin_transaction/commit/rollback.
    /// Due to the generic nature of this method, it makes the Database trait not object-safe,
    /// preventing use in trait objects (e.g., `Box<dyn Database>`).
    ///
    /// For applications needing connection pooling or trait objects, use the manual
    /// begin_transaction/commit/rollback methods instead.
    ///
    /// # Thread Safety
    /// Safe to call concurrently. Each invocation gets its own transaction scope.
    async fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: for<'a> FnOnce(
                &'a Self,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>,
            > + Send,
        T: Send,
    {
        self.begin_transaction().await?;

        match f(self).await {
            Ok(result) => {
                self.commit().await?;
                Ok(result)
            }
            Err(e) => {
                let _ = self.rollback().await;
                Err(e)
            }
        }
    }
}

/// Object-safe version of the Database trait
///
/// This trait is identical to Database but without the generic transaction() method,
/// making it object-safe and suitable for use in trait objects like `Box<dyn DatabaseObject>`.
///
/// Use this when you need connection pooling or dynamic dispatch.
///
/// # Example
/// ```ignore
/// let db: Box<dyn DatabaseObject> = Box::new(SqliteDatabase::new());
/// let pool: Vec<Box<dyn DatabaseObject>> = vec![db];
/// ```
#[async_trait]
pub trait DatabaseObject: Send + Sync {
    /// Get the database type
    fn database_type(&self) -> DatabaseType;

    /// Connect to the database with the given connection string
    async fn connect(&self, connection_string: &str) -> Result<()>;

    /// Check if connected to the database
    fn is_connected(&self) -> bool;

    /// Disconnect from the database
    async fn disconnect(&self) -> Result<()>;

    /// Execute a query that doesn't return results
    async fn execute(&self, query: &str) -> Result<u64>;

    /// Execute a SELECT query and return results
    async fn query(&self, query: &str) -> Result<DatabaseResult>;

    /// Execute a query with parameters (prepared statement)
    async fn query_with_params(
        &self,
        query: &str,
        params: &[DatabaseValue],
    ) -> Result<DatabaseResult>;

    /// Execute a query with parameters that doesn't return results
    async fn execute_with_params(&self, query: &str, params: &[DatabaseValue]) -> Result<u64>;

    /// Begin a transaction
    async fn begin_transaction(&self) -> Result<()>;

    /// Commit the current transaction
    async fn commit(&self) -> Result<()>;

    /// Rollback the current transaction
    async fn rollback(&self) -> Result<()>;

    /// Check if currently in a transaction
    fn in_transaction(&self) -> bool;
}

/// Blanket implementation of DatabaseObject for all types implementing Database
///
/// This allows any Database implementation to automatically work as a DatabaseObject
#[async_trait]
impl<T: Database> DatabaseObject for T {
    fn database_type(&self) -> DatabaseType {
        Database::database_type(self)
    }

    async fn connect(&self, connection_string: &str) -> Result<()> {
        Database::connect(self, connection_string).await
    }

    fn is_connected(&self) -> bool {
        Database::is_connected(self)
    }

    async fn disconnect(&self) -> Result<()> {
        Database::disconnect(self).await
    }

    async fn execute(&self, query: &str) -> Result<u64> {
        Database::execute(self, query).await
    }

    async fn query(&self, query: &str) -> Result<DatabaseResult> {
        Database::query(self, query).await
    }

    async fn query_with_params(
        &self,
        query: &str,
        params: &[DatabaseValue],
    ) -> Result<DatabaseResult> {
        Database::query_with_params(self, query, params).await
    }

    async fn execute_with_params(&self, query: &str, params: &[DatabaseValue]) -> Result<u64> {
        Database::execute_with_params(self, query, params).await
    }

    async fn begin_transaction(&self) -> Result<()> {
        Database::begin_transaction(self).await
    }

    async fn commit(&self) -> Result<()> {
        Database::commit(self).await
    }

    async fn rollback(&self) -> Result<()> {
        Database::rollback(self).await
    }

    fn in_transaction(&self) -> bool {
        Database::in_transaction(self)
    }
}

/// Trait for database connection pooling
///
/// Uses DatabaseObject instead of Database to enable trait object usage.
/// This allows connection pooling with dynamic dispatch.
///
/// # Example
/// ```ignore
/// let pool = MyConnectionPool::new(10); // 10 connections
/// let conn = pool.acquire().await?;
/// conn.execute("SELECT 1").await?;
/// pool.release(conn).await?;
/// ```
#[async_trait]
pub trait ConnectionPool: Send + Sync {
    /// Get a connection from the pool
    ///
    /// Blocks until a connection is available or timeout occurs.
    async fn acquire(&self) -> Result<Box<dyn DatabaseObject>>;

    /// Return a connection to the pool
    ///
    /// The connection should be in a clean state (no active transaction).
    async fn release(&self, connection: Box<dyn DatabaseObject>) -> Result<()>;

    /// Get the current pool size (total connections)
    fn size(&self) -> usize;

    /// Get the number of active (borrowed) connections
    fn active_count(&self) -> usize;

    /// Get the number of idle (available) connections
    fn idle_count(&self) -> usize;

    /// Close all connections and shut down the pool
    async fn close(&self) -> Result<()>;
}

/// Database connection builder
pub struct ConnectionBuilder {
    db_type: DatabaseType,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    username: Option<String>,
    password: Option<String>,
    options: std::collections::HashMap<String, String>,
}

impl ConnectionBuilder {
    /// Create a new connection builder for the specified database type
    pub fn new(db_type: DatabaseType) -> Self {
        Self {
            db_type,
            host: None,
            port: None,
            database: None,
            username: None,
            password: None,
            options: std::collections::HashMap::new(),
        }
    }

    /// Set the database host
    pub fn host<S: Into<String>>(mut self, host: S) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Set the database port
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the database name
    pub fn database<S: Into<String>>(mut self, database: S) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set the username
    pub fn username<S: Into<String>>(mut self, username: S) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password
    pub fn password<S: Into<String>>(mut self, password: S) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Add a custom option
    pub fn option<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// Build the connection string
    pub fn build_connection_string(&self) -> String {
        match self.db_type {
            DatabaseType::Sqlite => self
                .database
                .clone()
                .unwrap_or_else(|| ":memory:".to_string()),
            DatabaseType::Postgres => {
                let mut parts = Vec::new();
                if let Some(host) = &self.host {
                    parts.push(format!("host={}", host));
                }
                if let Some(port) = self.port {
                    parts.push(format!("port={}", port));
                }
                if let Some(database) = &self.database {
                    parts.push(format!("dbname={}", database));
                }
                if let Some(username) = &self.username {
                    parts.push(format!("user={}", username));
                }
                if let Some(password) = &self.password {
                    parts.push(format!("password={}", password));
                }
                for (key, value) in &self.options {
                    parts.push(format!("{}={}", key, value));
                }
                parts.join(" ")
            }
            DatabaseType::Mysql => {
                let host = self.host.as_deref().unwrap_or("localhost");
                let port = self.port.unwrap_or(3306);
                let database = self.database.as_deref().unwrap_or("");
                let username = self.username.as_deref().unwrap_or("root");
                let password = self.password.as_deref().unwrap_or("");
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    username, password, host, port, database
                )
            }
            DatabaseType::Redis => {
                let host = self.host.as_deref().unwrap_or("localhost");
                let port = self.port.unwrap_or(6379);
                format!("redis://{}:{}", host, port)
            }
            DatabaseType::Mongodb => {
                let host = self.host.as_deref().unwrap_or("localhost");
                let port = self.port.unwrap_or(27017);
                format!("mongodb://{}:{}", host, port)
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_builder_sqlite() {
        let builder = ConnectionBuilder::new(DatabaseType::Sqlite).database("test.db");
        assert_eq!(builder.build_connection_string(), "test.db");

        let builder = ConnectionBuilder::new(DatabaseType::Sqlite);
        assert_eq!(builder.build_connection_string(), ":memory:");
    }

    #[test]
    fn test_connection_builder_postgres() {
        let builder = ConnectionBuilder::new(DatabaseType::Postgres)
            .host("localhost")
            .port(5432)
            .database("mydb")
            .username("user")
            .password("pass");

        let conn_str = builder.build_connection_string();
        assert!(conn_str.contains("host=localhost"));
        assert!(conn_str.contains("port=5432"));
        assert!(conn_str.contains("dbname=mydb"));
        assert!(conn_str.contains("user=user"));
        assert!(conn_str.contains("password=pass"));
    }

    #[test]
    fn test_connection_builder_mysql() {
        let builder = ConnectionBuilder::new(DatabaseType::Mysql)
            .host("localhost")
            .port(3306)
            .database("mydb")
            .username("user")
            .password("pass");

        let conn_str = builder.build_connection_string();
        assert_eq!(conn_str, "mysql://user:pass@localhost:3306/mydb");
    }
}
