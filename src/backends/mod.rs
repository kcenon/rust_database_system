//! Database backend implementations
//!
//! This module contains concrete implementations of the Database trait
//! for various database systems.

#[cfg(feature = "sqlite")]
pub mod pooled_sqlite;
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub use pooled_sqlite::{PoolConfig, PoolStats, PooledSqliteDatabase, PooledTransaction};
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDatabase;

#[cfg(feature = "postgres")]
pub use postgres::PostgresDatabase;
