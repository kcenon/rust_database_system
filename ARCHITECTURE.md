# Rust Database System Architecture

> **Languages**: English | [한국어](./ARCHITECTURE.ko.md)

## Overview

The Rust Database System provides a unified abstraction layer over multiple database backends, offering type-safe operations, transaction management, and async/await support.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                  Application Layer                       │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Database Trait (core/database.rs)          │
│  - connect()                                             │
│  - execute()                                             │
│  - query()                                               │
│  - begin_transaction() / commit() / rollback()          │
└────────────────────┬────────────────────────────────────┘
                     │
          ┌──────────┼──────────┐
          │          │          │
          ▼          ▼          ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐
    │ SQLite  │ │Postgres │ │  MySQL  │
    │ Backend │ │ Backend │ │ Backend │
    └─────────┘ └─────────┘ └─────────┘
```

## Core Components

### 1. Database Trait (`core/database.rs`)

The central abstraction defining all database operations:

```rust
#[async_trait]
pub trait Database: Send + Sync {
    fn database_type(&self) -> DatabaseType;
    async fn connect(&mut self, connection_string: &str) -> Result<()>;
    async fn execute(&mut self, query: &str) -> Result<u64>;
    async fn query(&mut self, query: &str) -> Result<DatabaseResult>;
    async fn begin_transaction(&mut self) -> Result<()>;
    async fn commit(&mut self) -> Result<()>;
    async fn rollback(&mut self) -> Result<()>;
}
```

**Design Decisions:**
- `async_trait` for async/await support
- `Send + Sync` bounds for thread safety
- Mutable references allow state management (connections, transactions)
- Result type for comprehensive error handling

### 2. Value System (`core/value.rs`)

Type-safe value representation across different databases:

```rust
pub enum DatabaseValue {
    Null,
    Bool(bool),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    Bytes(Vec<u8>),
    Timestamp(i64),
}
```

**Features:**
- Type conversions with `From` trait implementations
- Safe extraction methods (`as_int()`, `as_string()`, etc.)
- Null handling
- Cross-database type mapping

### 3. Error Handling (`core/error.rs`)

Comprehensive error types using `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Query execution error: {0}")]
    QueryError(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    // Backend-specific errors
    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
}
```

### 4. Backend Implementations (`backends/`)

Each backend implements the `Database` trait:

#### SQLite Backend (`backends/sqlite.rs`)

```rust
pub struct SqliteDatabase {
    connection: Arc<Mutex<Option<Connection>>>,
    in_transaction: Arc<Mutex<bool>>,
}
```

**Thread Safety:**
- `Arc<Mutex<>>` pattern for shared connection
- Separate transaction state tracking
- `parking_lot::Mutex` for better performance

**Key Features:**
- Bundled SQLite (no external dependencies)
- Foreign key support enabled by default
- Prepared statement support

## Data Flow

### Query Execution Flow

```
Application
    │
    ├─> db.query("SELECT * FROM users")
    │
    ▼
Database Trait
    │
    ├─> Backend-specific implementation
    │
    ▼
SQLite Backend
    │
    ├─> Parse query with rusqlite
    ├─> Execute query
    ├─> Convert rusqlite::Row to DatabaseRow
    │
    ▼
DatabaseResult (Vec<DatabaseRow>)
    │
    ▼
Application receives results
```

### Transaction Flow

```
1. begin_transaction()
   │
   ├─> Execute "BEGIN TRANSACTION"
   ├─> Set in_transaction = true
   │
2. execute() operations
   │
   ├─> Operations within transaction
   │
3a. commit()                    3b. rollback()
    │                               │
    ├─> Execute "COMMIT"           ├─> Execute "ROLLBACK"
    ├─> Set in_transaction=false   ├─> Set in_transaction=false
```

## Connection Management

### Connection Builder Pattern

```rust
let conn_str = ConnectionBuilder::new(DatabaseType::Postgres)
    .host("localhost")
    .port(5432)
    .database("mydb")
    .username("user")
    .password("pass")
    .build_connection_string();
```

**Benefits:**
- Type-safe connection string building
- Database-specific format handling
- Reduces connection string errors

### Future: Connection Pooling

Planned architecture for connection pooling:

```rust
pub trait ConnectionPool: Send + Sync {
    async fn acquire(&self) -> Result<Arc<dyn Database>>;
    async fn release(&self, connection: Arc<dyn Database>) -> Result<()>;
    fn size(&self) -> usize;
    fn active_count(&self) -> usize;
}
```

## Type Mapping

### Rust ↔ Database Type Mapping

| Rust Type | SQLite Type | PostgreSQL Type | MySQL Type |
|-----------|-------------|-----------------|------------|
| `bool` | `INTEGER` | `BOOLEAN` | `TINYINT(1)` |
| `i32` | `INTEGER` | `INT` | `INT` |
| `i64` | `INTEGER` | `BIGINT` | `BIGINT` |
| `f32` | `REAL` | `REAL` | `FLOAT` |
| `f64` | `REAL` | `DOUBLE PRECISION` | `DOUBLE` |
| `String` | `TEXT` | `VARCHAR/TEXT` | `VARCHAR/TEXT` |
| `Vec<u8>` | `BLOB` | `BYTEA` | `BLOB` |
| `Option<T>` | `NULL` | `NULL` | `NULL` |

## Error Handling Strategy

### Error Propagation

```rust
pub type Result<T> = std::result::Result<T, DatabaseError>;

// Application code
async fn example() -> Result<()> {
    let mut db = SqliteDatabase::new();
    db.connect(":memory:").await?;  // Auto-converts rusqlite::Error
    db.execute("CREATE TABLE users (id INTEGER)").await?;
    Ok(())
}
```

### Error Context

Errors include context for debugging:
- Connection errors: include connection string (sanitized)
- Query errors: include query text (truncated if long)
- Type errors: include expected and actual types

## Testing Strategy

### Unit Tests

Each component has unit tests:
- Value conversions
- Error creation
- Connection string building

### Integration Tests

Backend-specific tests:
- Connection lifecycle
- Query execution
- Transaction atomicity
- Type conversion round-trips

### Test Example

```rust
#[tokio::test]
async fn test_transaction_rollback() {
    let mut db = SqliteDatabase::new();
    db.connect(":memory:").await.unwrap();

    db.execute("CREATE TABLE test (id INTEGER)").await.unwrap();

    db.begin_transaction().await.unwrap();
    db.execute("INSERT INTO test VALUES (1)").await.unwrap();
    db.rollback().await.unwrap();

    let results = db.query("SELECT * FROM test").await.unwrap();
    assert_eq!(results.len(), 0); // Rollback successful
}
```

## Performance Considerations

### Async Operations

- All I/O operations are async
- Tokio runtime for async execution
- Non-blocking database operations

### Memory Management

- `Arc` for shared ownership without clones
- `Mutex` only locks during actual I/O
- Efficient string handling with `String` instead of `&str` for values

### Query Optimization

- Prepared statements support (reduces parsing overhead)
- Batch operations planned for future
- Connection pooling planned for future

## Future Enhancements

### Planned Features

1. **Connection Pooling**
   - Reuse connections
   - Configurable pool size
   - Connection health checks

2. **Query Builder**
   ```rust
   db.select()
       .from("users")
       .where_eq("status", "active")
       .execute()
       .await?;
   ```

3. **ORM Features**
   - Struct-to-table mapping
   - Automatic migrations
   - Relations support

4. **Additional Backends**
   - PostgreSQL (planned)
   - MySQL (planned)
   - MongoDB (planned)
   - Redis (planned)

## Design Philosophy

### Core Principles

1. **Safety First**: Use Rust's type system to prevent errors at compile time
2. **Simple API**: Common operations should be simple and intuitive
3. **Backend Agnostic**: Write once, run on any supported database
4. **Performance**: Zero-cost abstractions where possible
5. **Async Native**: Built for modern async Rust applications

### Trade-offs

- **Flexibility vs. Simplicity**: Chose simpler API over maximum flexibility
- **Type Safety vs. Dynamic Queries**: Compile-time safety over runtime flexibility
- **Performance vs. Ergonomics**: Slightly more allocations for better API

## Integration Examples

### With Web Frameworks

```rust
// Axum integration
async fn get_users(
    State(db): State<Arc<Mutex<SqliteDatabase>>>
) -> Result<Json<Vec<User>>, StatusCode> {
    let mut db = db.lock().await;
    let results = db.query("SELECT * FROM users").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert to User structs
    Ok(Json(users))
}
```

### With Other Rust Systems

```rust
// Integration with rust_logger_system
let logger = Logger::new();
let mut db = SqliteDatabase::new();

match db.connect(":memory:").await {
    Ok(_) => logger.info("Database connected"),
    Err(e) => logger.error(format!("Database error: {}", e)),
}
```

---

*Architecture Document Version 1.0*
*Last Updated: 2025-10-16*
