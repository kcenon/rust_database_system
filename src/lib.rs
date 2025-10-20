//! # Rust Database System
//!
//! A production-ready, high-performance Rust database abstraction layer designed to provide
//! unified access to multiple database backends with advanced features including connection
//! pooling, transaction management, and type-safe query building.
//!
//! This is a Rust implementation of the [database_system](https://github.com/kcenon/database_system)
//! project, providing the same functionality with Rust's safety guarantees and performance benefits.
//!
//! ## Features
//!
//! - **Type Safety**: Strongly-typed value system with compile-time checks
//! - **Thread Safety**: Built-in thread-safe operations using `parking_lot`
//! - **Async Support**: Async/await support with Tokio
//! - **Multiple Backends**: Support for SQLite, PostgreSQL, MySQL, MongoDB, and Redis
//! - **Transaction Management**: Full ACID transaction support
//! - **Connection Pooling**: Efficient connection management (planned)
//! - **Cross-Platform**: Works on Windows, Linux, and macOS
//!
//! ## Supported Databases
//!
//! | Database | Status | Features |
//! |----------|--------|----------|
//! | SQLite | ✅ Implemented | Full support, bundled |
//! | PostgreSQL | 🔄 Planned | Async, prepared statements |
//! | MySQL | 🔄 Planned | Async, transactions |
//! | MongoDB | 🔄 Planned | Document operations |
//! | Redis | 🔄 Planned | Key-value operations |
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rust_database_system = { version = "0.1", features = ["sqlite"] }
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ### Basic Usage
//!
//! ```rust,no_run
//! use rust_database_system::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create a new SQLite database
//!     let mut db = SqliteDatabase::new();
//!
//!     // Connect to database
//!     db.connect(":memory:").await?;
//!
//!     // Create table
//!     db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)").await?;
//!
//!     // Insert data
//!     db.execute("INSERT INTO users (name) VALUES ('Alice')").await?;
//!
//!     // Query data
//!     let results = db.query("SELECT * FROM users").await?;
//!     for row in results {
//!         if let Some(name) = row.get("name") {
//!             println!("User: {}", name.as_string());
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Working with Transactions
//!
//! ```rust,no_run
//! use rust_database_system::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut db = SqliteDatabase::new();
//!     db.connect(":memory:").await?;
//!
//!     db.execute("CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance REAL)").await?;
//!
//!     // Use transaction
//!     db.begin_transaction().await?;
//!
//!     match db.execute("INSERT INTO accounts (balance) VALUES (100.0)").await {
//!         Ok(_) => db.commit().await?,
//!         Err(e) => {
//!             db.rollback().await?;
//!             return Err(e);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Project Structure
//!
//! ```text
//! rust_database_system/
//! ├── src/
//! │   ├── core/              # Core types and traits
//! │   │   ├── database.rs    # Database trait
//! │   │   ├── database_types.rs  # Database type enum
//! │   │   ├── error.rs       # Error types
//! │   │   ├── value.rs       # Value types
//! │   │   └── mod.rs
//! │   ├── backends/          # Database backend implementations
//! │   │   ├── sqlite.rs      # SQLite implementation
//! │   │   └── mod.rs
//! │   └── lib.rs
//! ├── examples/              # Example programs
//! ├── tests/                 # Integration tests
//! ├── Cargo.toml
//! └── README.md
//! ```
//!
//! ## Comparison with C++ Version
//!
//! | Feature | C++ Version | Rust Version |
//! |---------|-------------|--------------|
//! | Type Safety | ✓ (C++20) | ✓ (Rust) |
//! | Thread Safety | Manual (mutex) | Automatic (Arc+Mutex) |
//! | Memory Safety | Manual (smart pointers) | Automatic (ownership) |
//! | Async Support | C++20 coroutines | Tokio async/await |
//! | Connection Pooling | ✓ | Planned |
//! | Performance | High | High |

/// Core database system types and traits
pub mod core;

/// Database backend implementations
pub mod backends;

/// Prelude for convenient imports
///
/// ```rust
/// use rust_database_system::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let mut db = SqliteDatabase::new();
///     db.connect(":memory:").await?;
///     Ok(())
/// }
/// ```
pub mod prelude {
    pub use crate::core::{
        ConnectionBuilder, Database, DatabaseError, DatabaseResult, DatabaseRow, DatabaseType,
        DatabaseValue, Result, TransactionGuard,
    };

    #[cfg(feature = "sqlite")]
    pub use crate::backends::SqliteDatabase;
}

// Re-export at root level for convenience
pub use core::{
    ConnectionBuilder, Database, DatabaseError, DatabaseResult, DatabaseRow, DatabaseType,
    DatabaseValue, Result, TransactionGuard,
};

#[cfg(feature = "sqlite")]
pub use backends::SqliteDatabase;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        use prelude::*;

        let db_type = DatabaseType::Sqlite;
        assert_eq!(db_type.to_str(), "sqlite");
        assert!(db_type.is_sql());
    }

    #[test]
    fn test_value_conversions() {
        use prelude::*;

        let val: DatabaseValue = 42.into();
        assert_eq!(val.as_int(), Some(42));

        let val: DatabaseValue = "test".into();
        assert_eq!(val.as_string(), "test");

        let val: DatabaseValue = true.into();
        assert_eq!(val.as_bool(), Some(true));
    }
}
