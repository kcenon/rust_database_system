//! Integration tests for database system
//!
//! These tests verify that the database system works correctly across different scenarios:
//! - Concurrent access
//! - Transaction handling
//! - Error recovery
//! - Timeout protection

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use rust_database_system::backends::sqlite::SqliteDatabase;
    use rust_database_system::core::database::Database;
    use rust_database_system::core::value::DatabaseValue;
    use std::sync::Arc;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_concurrent_access() {
        // Test that multiple tasks can access the database concurrently
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)")
            .await
            .expect("Failed to create table");

        // Spawn multiple tasks that insert concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let db_clone = Arc::clone(&db);
            let handle = tokio::spawn(async move {
                // Use parameterized queries to prevent SQL injection
                let params = vec![DatabaseValue::Int(i), DatabaseValue::Int(i * 10)];
                db_clone
                    .execute_with_params("INSERT INTO test (id, value) VALUES (?, ?)", &params)
                    .await
            });
            handles.push(handle);
        }

        // Wait for all inserts to complete
        for handle in handles {
            handle.await.expect("Task panicked").expect("Insert failed");
        }

        // Verify all rows were inserted
        let results = db
            .query("SELECT COUNT(*) as count FROM test")
            .await
            .expect("Query failed");
        assert_eq!(results.len(), 1);

        let count = results[0].get("count").expect("Column not found").as_long();
        assert_eq!(count, Some(10));
    }

    #[tokio::test]
    async fn test_transaction_rollback_on_error() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .await
            .expect("Failed to create table");

        // Insert initial data
        db.execute("INSERT INTO test (id, value) VALUES (1, 'initial')")
            .await
            .expect("Failed to insert");

        // Start transaction
        db.begin_transaction()
            .await
            .expect("Failed to begin transaction");

        // Insert data in transaction
        db.execute("INSERT INTO test (id, value) VALUES (2, 'transactional')")
            .await
            .expect("Failed to insert in transaction");

        // Rollback
        db.rollback().await.expect("Failed to rollback");

        // Verify rollback worked - only initial data should exist
        let results = db
            .query("SELECT COUNT(*) as count FROM test")
            .await
            .expect("Query failed");
        let count = results[0].get("count").expect("Column not found").as_long();
        assert_eq!(count, Some(1));
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .await
            .expect("Failed to create table");

        // Start transaction
        db.begin_transaction()
            .await
            .expect("Failed to begin transaction");

        // Insert data
        db.execute("INSERT INTO test (id, value) VALUES (1, 'committed')")
            .await
            .expect("Failed to insert");

        // Commit
        db.commit().await.expect("Failed to commit");

        // Verify data persisted
        let results = db
            .query("SELECT value FROM test WHERE id = 1")
            .await
            .expect("Query failed");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0]
                .get("value")
                .expect("Column not found")
                .as_string(),
            "committed"
        );
    }

    #[tokio::test]
    async fn test_nested_transaction_prevention() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        // Start first transaction
        db.begin_transaction()
            .await
            .expect("Failed to begin transaction");

        // Try to start nested transaction - should fail
        let result = db.begin_transaction().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Already in a transaction"));

        // Cleanup
        db.rollback().await.expect("Failed to rollback");
    }

    #[tokio::test]
    async fn test_parameterized_query() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
            .await
            .expect("Failed to create table");

        // Insert with parameters
        let params = vec![
            DatabaseValue::String("Alice".to_string()),
            DatabaseValue::Int(30),
        ];

        db.execute_with_params("INSERT INTO users (name, age) VALUES (?, ?)", &params)
            .await
            .expect("Failed to insert");

        // Query with parameters
        let query_params = vec![DatabaseValue::String("Alice".to_string())];
        let results = db
            .query_with_params("SELECT age FROM users WHERE name = ?", &query_params)
            .await
            .expect("Query failed");

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].get("age").expect("Column not found").as_int(),
            Some(30)
        );
    }

    #[tokio::test]
    async fn test_concurrent_transactions() {
        // Test that transactions from different Arc references work correctly
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)")
            .await
            .expect("Failed to create table");

        // Transaction 1: commit
        let db1 = Arc::clone(&db);
        let handle1 = tokio::spawn(async move {
            db1.begin_transaction().await.expect("Failed to begin");
            db1.execute("INSERT INTO test (id, value) VALUES (1, 100)")
                .await
                .expect("Failed to insert");
            tokio::time::sleep(Duration::from_millis(50)).await;
            db1.commit().await.expect("Failed to commit");
        });

        // Wait for first transaction to finish
        handle1.await.expect("Task panicked");

        // Transaction 2: separate transaction
        db.begin_transaction()
            .await
            .expect("Failed to begin second transaction");
        db.execute("INSERT INTO test (id, value) VALUES (2, 200)")
            .await
            .expect("Failed to insert");
        db.commit().await.expect("Failed to commit");

        // Verify both inserts succeeded
        let results = db
            .query("SELECT COUNT(*) as count FROM test")
            .await
            .expect("Query failed");
        let count = results[0].get("count").expect("Column not found").as_long();
        assert_eq!(count, Some(2));
    }

    #[tokio::test]
    async fn test_connection_reuse() {
        // Test that the database can be disconnected and reconnected
        let db = Arc::new(SqliteDatabase::new());

        // First connection
        db.connect(":memory:").await.expect("Failed to connect");
        assert!(db.is_connected());

        // Disconnect
        db.disconnect().await.expect("Failed to disconnect");
        assert!(!db.is_connected());

        // Reconnect
        db.connect(":memory:").await.expect("Failed to reconnect");
        assert!(db.is_connected());

        // Should be able to execute queries
        db.execute("CREATE TABLE test (id INTEGER)")
            .await
            .expect("Failed to execute after reconnect");
    }

    #[tokio::test]
    async fn test_error_handling_invalid_sql() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        // Invalid SQL should return error
        let result = db.execute("INVALID SQL STATEMENT").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_handling_not_connected() {
        let db = Arc::new(SqliteDatabase::new());

        // Operations on disconnected database should fail
        let result = db.execute("SELECT 1").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not connected"));
    }

    #[tokio::test]
    async fn test_null_values() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .await
            .expect("Failed to create table");

        // Insert NULL value
        let params = vec![DatabaseValue::Int(1), DatabaseValue::Null];
        db.execute_with_params("INSERT INTO test (id, value) VALUES (?, ?)", &params)
            .await
            .expect("Failed to insert NULL");

        // Query and verify NULL
        let results = db
            .query("SELECT value FROM test WHERE id = 1")
            .await
            .expect("Query failed");
        assert_eq!(results.len(), 1);

        let value = results[0].get("value").expect("Column not found");
        assert!(matches!(value, DatabaseValue::Null));
    }

    #[tokio::test]
    async fn test_blob_data() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.expect("Failed to connect");

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, data BLOB)")
            .await
            .expect("Failed to create table");

        // Insert binary data
        let binary_data = vec![0u8, 1, 2, 3, 255];
        let params = vec![
            DatabaseValue::Int(1),
            DatabaseValue::Bytes(binary_data.clone()),
        ];

        db.execute_with_params("INSERT INTO test (id, data) VALUES (?, ?)", &params)
            .await
            .expect("Failed to insert blob");

        // Query and verify
        let results = db
            .query("SELECT data FROM test WHERE id = 1")
            .await
            .expect("Query failed");
        assert_eq!(results.len(), 1);

        let retrieved = results[0].get("data").expect("Column not found").as_bytes();
        assert_eq!(retrieved, Some(&binary_data[..]));
    }
}

#[cfg(feature = "postgres")]
mod postgres_tests {
    use rust_database_system::backends::postgres::PostgresDatabase;
    use rust_database_system::core::database::Database;
    use std::sync::Arc;

    // Note: These tests require a running PostgreSQL instance
    // Set POSTGRES_URL environment variable to run these tests

    fn get_postgres_url() -> Option<String> {
        std::env::var("POSTGRES_URL").ok()
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test --features postgres -- --ignored
    async fn test_postgres_connection() {
        let url = match get_postgres_url() {
            Some(url) => url,
            None => {
                println!("Skipping test: POSTGRES_URL not set");
                return;
            }
        };

        let db = Arc::new(PostgresDatabase::new());
        let result = db.connect(&url).await;
        assert!(result.is_ok());
        assert!(db.is_connected());
    }
}
