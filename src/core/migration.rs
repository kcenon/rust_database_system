//! Database migration system
//!
//! Provides tools for managing database schema versions with up/down migrations.
//!
//! # Example
//!
//! ```rust
//! use rust_database_system::core::{Database, Migration, MigrationManager};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create migrations
//! let migration1 = Migration::new(
//!     1,
//!     "create_users_table",
//!     "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
//!     "DROP TABLE users"
//! );
//!
//! let migration2 = Migration::new(
//!     2,
//!     "add_email_to_users",
//!     "ALTER TABLE users ADD COLUMN email TEXT",
//!     "ALTER TABLE users DROP COLUMN email"
//! );
//!
//! // Apply migrations
//! // let db: Box<dyn Database> = /* your database instance */;
//! // let mut manager = MigrationManager::new(db);
//! // manager.add_migration(migration1);
//! // manager.add_migration(migration2);
//! // manager.migrate().await?;
//! # Ok(())
//! # }
//! ```

use super::database::Database;
use super::error::{DatabaseError, Result};
use super::value::DatabaseValue;
use std::collections::BTreeMap;

/// Represents a single database migration
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version number (must be unique and sequential)
    version: i64,
    /// Human-readable name for this migration
    name: String,
    /// SQL to apply this migration (forward)
    up_sql: String,
    /// SQL to revert this migration (backward)
    down_sql: String,
}

impl Migration {
    /// Create a new migration
    ///
    /// # Arguments
    ///
    /// * `version` - Unique version number for this migration
    /// * `name` - Descriptive name for this migration
    /// * `up_sql` - SQL statements to apply the migration
    /// * `down_sql` - SQL statements to revert the migration
    ///
    /// # Example
    ///
    /// ```rust
    /// use rust_database_system::core::Migration;
    ///
    /// let migration = Migration::new(
    ///     1,
    ///     "create_users_table",
    ///     "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
    ///     "DROP TABLE users"
    /// );
    /// ```
    pub fn new(
        version: i64,
        name: impl Into<String>,
        up_sql: impl Into<String>,
        down_sql: impl Into<String>,
    ) -> Self {
        Self {
            version,
            name: name.into(),
            up_sql: up_sql.into(),
            down_sql: down_sql.into(),
        }
    }

    /// Get the migration version
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Get the migration name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the up SQL
    pub fn up_sql(&self) -> &str {
        &self.up_sql
    }

    /// Get the down SQL
    pub fn down_sql(&self) -> &str {
        &self.down_sql
    }
}

/// Migration status for a specific version
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationStatus {
    /// Migration has been applied
    Applied,
    /// Migration is pending
    Pending,
}

/// Manages database migrations
pub struct MigrationManager<D: Database> {
    db: D,
    migrations: BTreeMap<i64, Migration>,
    table_name: String,
}

impl<D: Database> MigrationManager<D> {
    /// Default name for the migrations tracking table
    pub const DEFAULT_TABLE_NAME: &'static str = "schema_migrations";

    /// Create a new migration manager
    ///
    /// # Arguments
    ///
    /// * `db` - Database instance to manage migrations for
    pub fn new(db: D) -> Self {
        Self {
            db,
            migrations: BTreeMap::new(),
            table_name: Self::DEFAULT_TABLE_NAME.to_string(),
        }
    }

    /// Create a migration manager with a custom tracking table name
    pub fn with_table_name(db: D, table_name: impl Into<String>) -> Self {
        Self {
            db,
            migrations: BTreeMap::new(),
            table_name: table_name.into(),
        }
    }

    /// Add a migration to the manager
    ///
    /// # Example
    ///
    /// ```rust
    /// use rust_database_system::core::{Database, Migration, MigrationManager};
    ///
    /// // FIXED: Added generic parameter <D: Database> to match struct signature
    /// # fn example<D: Database>(mut manager: MigrationManager<D>) {
    /// let migration = Migration::new(
    ///     1,
    ///     "create_users",
    ///     "CREATE TABLE users (id INTEGER PRIMARY KEY)",
    ///     "DROP TABLE users"
    /// );
    ///
    /// manager.add_migration(migration);
    /// # }
    /// ```
    pub fn add_migration(&mut self, migration: Migration) {
        self.migrations.insert(migration.version, migration);
    }

    /// Ensure the migrations tracking table exists
    async fn ensure_migrations_table(&self) -> Result<()> {
        let create_table_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                version INTEGER PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            )",
            self.table_name
        );

        self.db.execute(&create_table_sql).await?;
        Ok(())
    }

    /// Get all applied migration versions
    async fn get_applied_versions(&self) -> Result<Vec<i64>> {
        self.ensure_migrations_table().await?;

        let query = format!("SELECT version FROM {} ORDER BY version", self.table_name);
        let result = self.db.query(&query).await?;

        let mut versions = Vec::new();
        for row in &result {
            if let Some(DatabaseValue::Long(version)) = row.get("version") {
                versions.push(*version);
            }
        }

        Ok(versions)
    }

    /// Check if a specific migration version has been applied
    pub async fn is_applied(&self, version: i64) -> Result<bool> {
        let applied = self.get_applied_versions().await?;
        Ok(applied.contains(&version))
    }

    /// Get the current schema version (highest applied migration)
    pub async fn current_version(&self) -> Result<Option<i64>> {
        let applied = self.get_applied_versions().await?;
        Ok(applied.last().copied())
    }

    /// Get the status of all migrations
    pub async fn migration_status(&self) -> Result<BTreeMap<i64, MigrationStatus>> {
        let applied = self.get_applied_versions().await?;
        let mut status = BTreeMap::new();

        for version in self.migrations.keys() {
            let migration_status = if applied.contains(version) {
                MigrationStatus::Applied
            } else {
                MigrationStatus::Pending
            };
            status.insert(*version, migration_status);
        }

        Ok(status)
    }

    /// Apply all pending migrations
    ///
    /// # Example
    ///
    /// ```rust
    /// use rust_database_system::core::{Database, Migration, MigrationManager};
    ///
    /// // FIXED: Added generic parameter <D: Database> to match struct signature
    /// # async fn example<D: Database>(mut manager: MigrationManager<D>) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.add_migration(Migration::new(
    ///     1,
    ///     "create_users",
    ///     "CREATE TABLE users (id INTEGER PRIMARY KEY)",
    ///     "DROP TABLE users"
    /// ));
    ///
    /// manager.migrate().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn migrate(&self) -> Result<Vec<i64>> {
        self.ensure_migrations_table().await?;
        let applied = self.get_applied_versions().await?;
        let mut migrated = Vec::new();

        for (version, migration) in &self.migrations {
            if applied.contains(version) {
                continue; // Already applied
            }

            // Execute the up migration
            self.db.execute(migration.up_sql()).await?;

            // Record the migration
            let insert_sql = format!(
                "INSERT INTO {} (version, name, applied_at) VALUES (?, ?, ?)",
                self.table_name
            );

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            self.db
                .execute_with_params(
                    &insert_sql,
                    &[
                        DatabaseValue::Long(*version),
                        DatabaseValue::String(migration.name().to_string()),
                        DatabaseValue::Long(now),
                    ],
                )
                .await?;

            migrated.push(*version);
        }

        Ok(migrated)
    }

    /// Migrate to a specific version
    ///
    /// If the target version is lower than current, migrations will be rolled back.
    /// If higher, pending migrations will be applied.
    pub async fn migrate_to(&self, target_version: i64) -> Result<Vec<i64>> {
        let current = self.current_version().await?.unwrap_or(0);

        if target_version > current {
            // Migrate up
            self.migrate_up_to(target_version).await
        } else if target_version < current {
            // Migrate down
            self.rollback_to(target_version).await
        } else {
            Ok(Vec::new()) // Already at target version
        }
    }

    /// Apply pending migrations up to a specific version
    async fn migrate_up_to(&self, target_version: i64) -> Result<Vec<i64>> {
        self.ensure_migrations_table().await?;
        let applied = self.get_applied_versions().await?;
        let mut migrated = Vec::new();

        for (version, migration) in &self.migrations {
            if *version > target_version {
                break;
            }

            if applied.contains(version) {
                continue;
            }

            // Execute the up migration
            self.db.execute(migration.up_sql()).await?;

            // Record the migration
            let insert_sql = format!(
                "INSERT INTO {} (version, name, applied_at) VALUES (?, ?, ?)",
                self.table_name
            );

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            self.db
                .execute_with_params(
                    &insert_sql,
                    &[
                        DatabaseValue::Long(*version),
                        DatabaseValue::String(migration.name().to_string()),
                        DatabaseValue::Long(now),
                    ],
                )
                .await?;

            migrated.push(*version);
        }

        Ok(migrated)
    }

    /// Rollback migrations to a specific version
    async fn rollback_to(&self, target_version: i64) -> Result<Vec<i64>> {
        let applied = self.get_applied_versions().await?;
        let mut rolled_back = Vec::new();

        // Process in reverse order
        for version in applied.iter().rev() {
            if *version <= target_version {
                break;
            }

            let migration = self.migrations.get(version).ok_or_else(|| {
                DatabaseError::Migration(format!("Migration {} not found", version))
            })?;

            // Execute the down migration
            self.db.execute(migration.down_sql()).await?;

            // Remove the migration record
            let delete_sql = format!("DELETE FROM {} WHERE version = ?", self.table_name);
            self.db
                .execute_with_params(&delete_sql, &[DatabaseValue::Long(*version)])
                .await?;

            rolled_back.push(*version);
        }

        Ok(rolled_back)
    }

    /// Rollback the last N migrations
    ///
    /// # Example
    ///
    /// ```rust
    /// use rust_database_system::core::{Database, MigrationManager};
    ///
    /// // FIXED: Added generic parameter <D: Database> to match struct signature
    /// # async fn example<D: Database>(manager: MigrationManager<D>) -> Result<(), Box<dyn std::error::Error>> {
    /// // Rollback the last 2 migrations
    /// manager.rollback(2).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rollback(&self, steps: usize) -> Result<Vec<i64>> {
        let applied = self.get_applied_versions().await?;

        if steps == 0 || applied.is_empty() {
            return Ok(Vec::new());
        }

        let rollback_count = steps.min(applied.len());
        let mut rolled_back = Vec::new();

        for version in applied.iter().rev().take(rollback_count) {
            let migration = self.migrations.get(version).ok_or_else(|| {
                DatabaseError::Migration(format!("Migration {} not found", version))
            })?;

            // Execute the down migration
            self.db.execute(migration.down_sql()).await?;

            // Remove the migration record
            let delete_sql = format!("DELETE FROM {} WHERE version = ?", self.table_name);
            self.db
                .execute_with_params(&delete_sql, &[DatabaseValue::Long(*version)])
                .await?;

            rolled_back.push(*version);
        }

        Ok(rolled_back)
    }

    /// Get the list of pending migrations
    pub async fn pending_migrations(&self) -> Result<Vec<&Migration>> {
        let applied = self.get_applied_versions().await?;
        let pending: Vec<&Migration> = self
            .migrations
            .values()
            .filter(|m| !applied.contains(&m.version))
            .collect();

        Ok(pending)
    }

    /// Reset the database by rolling back all migrations
    pub async fn reset(&self) -> Result<Vec<i64>> {
        let applied = self.get_applied_versions().await?;
        self.rollback(applied.len()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::sqlite::SqliteDatabase;

    async fn create_test_db() -> SqliteDatabase {
        let db = SqliteDatabase::new();
        db.connect(":memory:").await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_migration_creation() {
        let migration = Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "DROP TABLE users",
        );

        assert_eq!(migration.version(), 1);
        assert_eq!(migration.name(), "create_users");
        assert_eq!(
            migration.up_sql(),
            "CREATE TABLE users (id INTEGER PRIMARY KEY)"
        );
        assert_eq!(migration.down_sql(), "DROP TABLE users");
    }

    #[tokio::test]
    async fn test_ensure_migrations_table() {
        let db = create_test_db().await;
        let manager = MigrationManager::new(db);

        manager.ensure_migrations_table().await.unwrap();

        // Verify table was created
        let result = manager
            .db
            .query("SELECT name FROM sqlite_master WHERE type='table' AND name='schema_migrations'")
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_migrate_single() {
        let db = create_test_db().await;
        let mut manager = MigrationManager::new(db);

        let migration = Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            "DROP TABLE users",
        );

        manager.add_migration(migration);

        let migrated = manager.migrate().await.unwrap();
        assert_eq!(migrated, vec![1]);

        let current_version = manager.current_version().await.unwrap();
        assert_eq!(current_version, Some(1));

        // Verify table was created
        let result = manager
            .db
            .query("SELECT name FROM sqlite_master WHERE type='table' AND name='users'")
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_migrate_multiple() {
        let db = create_test_db().await;
        let mut manager = MigrationManager::new(db);

        manager.add_migration(Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            "DROP TABLE users",
        ));

        manager.add_migration(Migration::new(
            2,
            "create_posts",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
            "DROP TABLE posts",
        ));

        let migrated = manager.migrate().await.unwrap();
        assert_eq!(migrated, vec![1, 2]);

        let current_version = manager.current_version().await.unwrap();
        assert_eq!(current_version, Some(2));
    }

    #[tokio::test]
    async fn test_rollback_single() {
        let db = create_test_db().await;
        let mut manager = MigrationManager::new(db);

        manager.add_migration(Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            "DROP TABLE users",
        ));

        manager.migrate().await.unwrap();

        let rolled_back = manager.rollback(1).await.unwrap();
        assert_eq!(rolled_back, vec![1]);

        let current_version = manager.current_version().await.unwrap();
        assert_eq!(current_version, None);

        // Verify table was dropped
        let result = manager
            .db
            .query("SELECT name FROM sqlite_master WHERE type='table' AND name='users'")
            .await
            .unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_migrate_to_version() {
        let db = create_test_db().await;
        let mut manager = MigrationManager::new(db);

        manager.add_migration(Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "DROP TABLE users",
        ));

        manager.add_migration(Migration::new(
            2,
            "create_posts",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
            "DROP TABLE posts",
        ));

        manager.add_migration(Migration::new(
            3,
            "create_comments",
            "CREATE TABLE comments (id INTEGER PRIMARY KEY)",
            "DROP TABLE comments",
        ));

        // Migrate to version 2
        let migrated = manager.migrate_to(2).await.unwrap();
        assert_eq!(migrated, vec![1, 2]);

        let current_version = manager.current_version().await.unwrap();
        assert_eq!(current_version, Some(2));

        // Rollback to version 1
        let rolled_back = manager.migrate_to(1).await.unwrap();
        assert_eq!(rolled_back, vec![2]);

        let current_version = manager.current_version().await.unwrap();
        assert_eq!(current_version, Some(1));
    }

    #[tokio::test]
    async fn test_migration_status() {
        let db = create_test_db().await;
        let mut manager = MigrationManager::new(db);

        manager.add_migration(Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "DROP TABLE users",
        ));

        manager.add_migration(Migration::new(
            2,
            "create_posts",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
            "DROP TABLE posts",
        ));

        // Before migration
        let status = manager.migration_status().await.unwrap();
        assert_eq!(status.get(&1), Some(&MigrationStatus::Pending));
        assert_eq!(status.get(&2), Some(&MigrationStatus::Pending));

        // Apply first migration
        manager.migrate_to(1).await.unwrap();

        let status = manager.migration_status().await.unwrap();
        assert_eq!(status.get(&1), Some(&MigrationStatus::Applied));
        assert_eq!(status.get(&2), Some(&MigrationStatus::Pending));

        // Apply second migration
        manager.migrate().await.unwrap();

        let status = manager.migration_status().await.unwrap();
        assert_eq!(status.get(&1), Some(&MigrationStatus::Applied));
        assert_eq!(status.get(&2), Some(&MigrationStatus::Applied));
    }

    #[tokio::test]
    async fn test_pending_migrations() {
        let db = create_test_db().await;
        let mut manager = MigrationManager::new(db);

        manager.add_migration(Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "DROP TABLE users",
        ));

        manager.add_migration(Migration::new(
            2,
            "create_posts",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
            "DROP TABLE posts",
        ));

        // All pending
        let pending = manager.pending_migrations().await.unwrap();
        assert_eq!(pending.len(), 2);

        // Apply first
        manager.migrate_to(1).await.unwrap();

        let pending = manager.pending_migrations().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].version(), 2);

        // Apply all
        manager.migrate().await.unwrap();

        let pending = manager.pending_migrations().await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_reset() {
        let db = create_test_db().await;
        let mut manager = MigrationManager::new(db);

        manager.add_migration(Migration::new(
            1,
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY)",
            "DROP TABLE users",
        ));

        manager.add_migration(Migration::new(
            2,
            "create_posts",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
            "DROP TABLE posts",
        ));

        manager.migrate().await.unwrap();

        let current_version = manager.current_version().await.unwrap();
        assert_eq!(current_version, Some(2));

        // Reset all
        let rolled_back = manager.reset().await.unwrap();
        assert_eq!(rolled_back, vec![2, 1]);

        let current_version = manager.current_version().await.unwrap();
        assert_eq!(current_version, None);
    }
}
