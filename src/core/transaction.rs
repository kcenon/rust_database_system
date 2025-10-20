//! Transaction guard for automatic rollback on drop
//!
//! This module provides RAII-style transaction management with automatic rollback.

use super::database::Database;
use super::error::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Transaction guard that automatically rolls back on drop if not committed
///
/// This provides RAII-style transaction management to prevent accidental
/// transaction leaks. If the guard is dropped without calling `commit()`,
/// the transaction will be automatically rolled back.
///
/// # Example
///
/// ```ignore
/// use rust_database_system::TransactionGuard;
///
/// async fn transfer_money(db: Arc<impl Database>) -> Result<()> {
///     let tx = TransactionGuard::begin(db).await?;
///
///     tx.execute("UPDATE accounts SET balance = balance - 100 WHERE id = 1").await?;
///     tx.execute("UPDATE accounts SET balance = balance + 100 WHERE id = 2").await?;
///
///     tx.commit().await?;  // Explicitly commit
///     Ok(())
/// }
///
/// // If commit() is not called (e.g., due to error), automatic rollback occurs
/// ```
pub struct TransactionGuard<D: Database + 'static> {
    db: Arc<D>,
    committed: AtomicBool,
    rolled_back: AtomicBool,
}

impl<D: Database + 'static> TransactionGuard<D> {
    /// Begin a new transaction
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database is not connected
    /// - A transaction is already active
    /// - Database operation fails
    pub async fn begin(db: Arc<D>) -> Result<Self> {
        db.begin_transaction().await?;

        Ok(Self {
            db,
            committed: AtomicBool::new(false),
            rolled_back: AtomicBool::new(false),
        })
    }

    /// Execute a query within the transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails
    pub async fn execute(&self, query: &str) -> Result<u64> {
        if self.committed.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot execute on committed transaction".to_string(),
            ));
        }
        if self.rolled_back.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot execute on rolled back transaction".to_string(),
            ));
        }

        self.db.execute(query).await
    }

    /// Execute a parameterized query within the transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails
    pub async fn execute_with_params(
        &self,
        query: &str,
        params: &[crate::core::DatabaseValue],
    ) -> Result<u64> {
        if self.committed.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot execute on committed transaction".to_string(),
            ));
        }
        if self.rolled_back.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot execute on rolled back transaction".to_string(),
            ));
        }

        self.db.execute_with_params(query, params).await
    }

    /// Query within the transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails
    pub async fn query(&self, query: &str) -> Result<crate::core::DatabaseResult> {
        if self.committed.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot query on committed transaction".to_string(),
            ));
        }
        if self.rolled_back.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot query on rolled back transaction".to_string(),
            ));
        }

        self.db.query(query).await
    }

    /// Query with parameters within the transaction
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails
    pub async fn query_with_params(
        &self,
        query: &str,
        params: &[crate::core::DatabaseValue],
    ) -> Result<crate::core::DatabaseResult> {
        if self.committed.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot query on committed transaction".to_string(),
            ));
        }
        if self.rolled_back.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot query on rolled back transaction".to_string(),
            ));
        }

        self.db.query_with_params(query, params).await
    }

    /// Commit the transaction
    ///
    /// After calling this method, the transaction is complete and the guard
    /// will not perform automatic rollback on drop.
    ///
    /// # Errors
    ///
    /// Returns an error if the commit fails
    pub async fn commit(self) -> Result<()> {
        if self.rolled_back.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot commit a rolled back transaction".to_string(),
            ));
        }

        self.db.commit().await?;
        self.committed.store(true, Ordering::Release);

        // FIXED: Removed mem::forget to prevent resource leak
        // Drop will see committed=true and skip rollback
        // This allows proper cleanup of Arc<D> and other resources

        Ok(())
    }

    /// Explicitly rollback the transaction
    ///
    /// This is rarely needed as rollback happens automatically on drop,
    /// but may be useful for explicit error handling.
    ///
    /// # Errors
    ///
    /// Returns an error if the rollback fails
    pub async fn rollback(self) -> Result<()> {
        if self.committed.load(Ordering::Acquire) {
            return Err(crate::core::DatabaseError::transaction(
                "Cannot rollback a committed transaction".to_string(),
            ));
        }

        self.db.rollback().await?;
        self.rolled_back.store(true, Ordering::Release);

        Ok(())
    }

    /// Check if the transaction has been committed
    pub fn is_committed(&self) -> bool {
        self.committed.load(Ordering::Acquire)
    }

    /// Check if the transaction has been rolled back
    pub fn is_rolled_back(&self) -> bool {
        self.rolled_back.load(Ordering::Acquire)
    }
}

impl<D: Database + 'static> Drop for TransactionGuard<D> {
    fn drop(&mut self) {
        // Auto-rollback if neither committed nor rolled back
        if !self.committed.load(Ordering::Acquire) && !self.rolled_back.load(Ordering::Acquire) {
            // SECURITY FIX: Use spawn_blocking to ensure rollback is properly scheduled
            // This is safer than fire-and-forget spawn, as it properly queues the blocking work

            let db = Arc::clone(&self.db);
            self.rolled_back.store(true, Ordering::Release);

            // Try to perform rollback if in tokio runtime
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                // Use spawn_blocking to schedule rollback on the blocking thread pool
                // This works with both single-threaded and multi-threaded runtimes
                handle.spawn_blocking(move || {
                    // Create a new runtime for the blocking operation
                    // This is necessary because we can't use async in spawn_blocking directly
                    // SAFETY FIX: Handle runtime creation failure gracefully instead of panicking
                    match tokio::runtime::Runtime::new() {
                        Ok(rt) => {
                            if let Err(e) = rt.block_on(db.rollback()) {
                                eprintln!(
                                    "[ERROR] TransactionGuard auto-rollback failed: {}. \
                                     Data may be in inconsistent state.", e
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "[CRITICAL] TransactionGuard cannot create runtime for auto-rollback: {}. \
                                 Transaction may leak. This typically indicates system resource exhaustion.", e
                            );
                        }
                    }
                });

                eprintln!(
                    "[WARNING] TransactionGuard dropped without explicit commit/rollback. \
                     Auto-rollback queued. Best practice: call commit() or rollback() explicitly."
                );
            } else {
                // Not in a tokio runtime - transaction will be rolled back on connection close
                eprintln!(
                    "[WARNING] TransactionGuard dropped outside tokio runtime. \
                     Transaction will be implicitly rolled back by database on connection close."
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::SqliteDatabase;

    #[tokio::test]
    async fn test_transaction_guard_commit() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .await
            .unwrap();

        // Transaction should commit
        {
            let tx = TransactionGuard::begin(Arc::clone(&db)).await.unwrap();
            tx.execute("INSERT INTO test (value) VALUES ('test1')")
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        // Verify data was committed
        let results = db.query("SELECT * FROM test").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_transaction_guard_rollback() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .await
            .unwrap();

        // Transaction should rollback
        {
            let tx = TransactionGuard::begin(Arc::clone(&db)).await.unwrap();
            tx.execute("INSERT INTO test (value) VALUES ('test1')")
                .await
                .unwrap();
            // Drop without commit - should rollback
        }

        // Give the async rollback task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Verify data was not committed
        let results = db.query("SELECT * FROM test").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_transaction_guard_explicit_rollback() {
        let db = Arc::new(SqliteDatabase::new());
        db.connect(":memory:").await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
            .await
            .unwrap();

        // Explicit rollback
        {
            let tx = TransactionGuard::begin(Arc::clone(&db)).await.unwrap();
            tx.execute("INSERT INTO test (value) VALUES ('test1')")
                .await
                .unwrap();
            tx.rollback().await.unwrap();
        }

        // Verify data was rolled back
        let results = db.query("SELECT * FROM test").await.unwrap();
        assert_eq!(results.len(), 0);
    }
}
