//! Database type definitions
//!
//! This module defines the types of databases supported by the system.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Supported database types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum DatabaseType {
    /// No database type specified
    #[default]
    None = 0,
    /// PostgreSQL database
    Postgres = 1,
    /// MySQL/MariaDB database
    Mysql = 2,
    /// SQLite database
    Sqlite = 3,
    /// Oracle database (future implementation)
    Oracle = 4,
    /// MongoDB database
    Mongodb = 5,
    /// Redis database
    Redis = 6,
}

impl DatabaseType {
    /// Convert database type to string representation
    pub fn to_str(&self) -> &'static str {
        match self {
            DatabaseType::None => "none",
            DatabaseType::Postgres => "postgres",
            DatabaseType::Mysql => "mysql",
            DatabaseType::Sqlite => "sqlite",
            DatabaseType::Oracle => "oracle",
            DatabaseType::Mongodb => "mongodb",
            DatabaseType::Redis => "redis",
        }
    }

    /// Parse database type from string (deprecated, use FromStr trait instead)
    #[deprecated(
        since = "0.1.0",
        note = "Use FromStr trait instead: s.parse::<DatabaseType>()"
    )]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Check if this database type is SQL-based
    pub fn is_sql(&self) -> bool {
        matches!(
            self,
            DatabaseType::Postgres
                | DatabaseType::Mysql
                | DatabaseType::Sqlite
                | DatabaseType::Oracle
        )
    }

    /// Check if this database type is NoSQL-based
    pub fn is_nosql(&self) -> bool {
        matches!(self, DatabaseType::Mongodb | DatabaseType::Redis)
    }

    /// Check if this database type supports transactions
    pub fn supports_transactions(&self) -> bool {
        matches!(
            self,
            DatabaseType::Postgres
                | DatabaseType::Mysql
                | DatabaseType::Sqlite
                | DatabaseType::Oracle
                | DatabaseType::Mongodb
        )
    }

    /// Check if this database type supports async operations
    pub fn supports_async(&self) -> bool {
        matches!(
            self,
            DatabaseType::Postgres
                | DatabaseType::Mysql
                | DatabaseType::Redis
                | DatabaseType::Mongodb
        )
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(DatabaseType::None),
            "postgres" | "postgresql" => Ok(DatabaseType::Postgres),
            "mysql" | "mariadb" => Ok(DatabaseType::Mysql),
            "sqlite" | "sqlite3" => Ok(DatabaseType::Sqlite),
            "oracle" => Ok(DatabaseType::Oracle),
            "mongodb" | "mongo" => Ok(DatabaseType::Mongodb),
            "redis" => Ok(DatabaseType::Redis),
            _ => Err(format!("Invalid database type: '{}'", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_type_to_str() {
        assert_eq!(DatabaseType::Postgres.to_str(), "postgres");
        assert_eq!(DatabaseType::Mysql.to_str(), "mysql");
        assert_eq!(DatabaseType::Sqlite.to_str(), "sqlite");
        assert_eq!(DatabaseType::Mongodb.to_str(), "mongodb");
        assert_eq!(DatabaseType::Redis.to_str(), "redis");
    }

    #[test]
    fn test_database_type_from_str() {
        assert_eq!(
            "postgres".parse::<DatabaseType>().ok(),
            Some(DatabaseType::Postgres)
        );
        assert_eq!(
            "postgresql".parse::<DatabaseType>().ok(),
            Some(DatabaseType::Postgres)
        );
        assert_eq!(
            "mysql".parse::<DatabaseType>().ok(),
            Some(DatabaseType::Mysql)
        );
        assert_eq!(
            "sqlite".parse::<DatabaseType>().ok(),
            Some(DatabaseType::Sqlite)
        );
        assert_eq!(
            "sqlite3".parse::<DatabaseType>().ok(),
            Some(DatabaseType::Sqlite)
        );
        assert_eq!(
            "mongodb".parse::<DatabaseType>().ok(),
            Some(DatabaseType::Mongodb)
        );
        assert_eq!(
            "redis".parse::<DatabaseType>().ok(),
            Some(DatabaseType::Redis)
        );
        assert_eq!("unknown".parse::<DatabaseType>().ok(), None);
    }

    #[test]
    fn test_database_type_is_sql() {
        assert!(DatabaseType::Postgres.is_sql());
        assert!(DatabaseType::Mysql.is_sql());
        assert!(DatabaseType::Sqlite.is_sql());
        assert!(!DatabaseType::Mongodb.is_sql());
        assert!(!DatabaseType::Redis.is_sql());
    }

    #[test]
    fn test_database_type_is_nosql() {
        assert!(!DatabaseType::Postgres.is_nosql());
        assert!(!DatabaseType::Mysql.is_nosql());
        assert!(!DatabaseType::Sqlite.is_nosql());
        assert!(DatabaseType::Mongodb.is_nosql());
        assert!(DatabaseType::Redis.is_nosql());
    }

    #[test]
    fn test_database_type_supports_transactions() {
        assert!(DatabaseType::Postgres.supports_transactions());
        assert!(DatabaseType::Mysql.supports_transactions());
        assert!(DatabaseType::Sqlite.supports_transactions());
        assert!(DatabaseType::Mongodb.supports_transactions());
        assert!(!DatabaseType::Redis.supports_transactions());
    }

    #[test]
    fn test_database_type_supports_async() {
        assert!(DatabaseType::Postgres.supports_async());
        assert!(DatabaseType::Mysql.supports_async());
        assert!(!DatabaseType::Sqlite.supports_async());
        assert!(DatabaseType::Redis.supports_async());
        assert!(DatabaseType::Mongodb.supports_async());
    }
}
