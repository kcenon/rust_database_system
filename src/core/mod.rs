//! Core database system types and traits
//!
//! This module provides the fundamental building blocks for the database system,
//! including error types, database traits, value types, and connection management.

pub mod database;
pub mod database_types;
pub mod error;
pub mod migration;
pub mod query_builder;
pub mod transaction;
pub mod value;

// Re-export commonly used types
pub use database::{ConnectionBuilder, Database};
pub use database_types::DatabaseType;
pub use error::{DatabaseError, Result};
pub use migration::{Migration, MigrationManager, MigrationStatus};
pub use query_builder::{
    DeleteBuilder, InsertBuilder, OrderDirection, SelectBuilder, UpdateBuilder,
};
pub use transaction::TransactionGuard;
pub use value::{DatabaseResult, DatabaseRow, DatabaseValue};
