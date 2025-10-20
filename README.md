# Rust Database System

> **Languages**: English | [한국어](./README.ko.md)

A production-ready, high-performance Rust database abstraction layer designed to provide unified access to multiple database backends with advanced features including connection pooling, transaction management, and type-safe query building.

This is a Rust implementation of the [database_system](https://github.com/kcenon/database_system) project, providing the same functionality with Rust's safety guarantees and performance benefits.

## Quality Status

- Verification: `cargo check`, `cargo test`(unit, integration, property, doc) ✅
- Critical fixes: `TransactionGuard::commit` 리소스 누수 제거, `MigrationManager` 문서 예제 보강
- Clippy: ✅ 0 warnings
- Production guidance: 트랜잭션 집중 환경에서도 안전하게 사용 가능

## Features

- **Type Safety**: Strongly-typed value system with compile-time checks
- **Thread Safety**: Built-in thread-safe operations using `parking_lot`
- **Memory Efficiency**: Efficient memory management with `Arc` and smart pointers
- **Async Support**: Full async/await support with Tokio runtime
- **Transaction Management**: ACID-compliant transaction support
- **Multiple Backends**: SQLite, PostgreSQL, MySQL, MongoDB, Redis support
- **Cross-Platform**: Works on Windows, Linux, and macOS
- **Zero-Cost Abstractions**: Performance comparable to native drivers

## Supported Databases

| Database | Status | Features | Async | Transactions |
|----------|--------|----------|-------|--------------|
| SQLite | ✅ Full | Bundled, WAL mode, FTS5 | ❌ | ✅ |
| PostgreSQL | 🔄 Planned | JSONB, Arrays, CTEs | ✅ | ✅ |
| MySQL | 🔄 Planned | Full-text search, Replication | ✅ | ✅ |
| MongoDB | 🔄 Planned | Documents, Aggregation | ✅ | ✅ |
| Redis | 🔄 Planned | All data types, Pub/Sub | ✅ | ❌ |

## Quick Start

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust_database_system = { version = "0.1", features = ["sqlite"] }
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use rust_database_system::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new SQLite database
    let mut db = SqliteDatabase::new();

    // Connect to database
    db.connect(":memory:").await?;

    // Create table
    db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)").await?;

    // Insert data
    db.execute("INSERT INTO users (name, age) VALUES ('Alice', 30)").await?;
    db.execute("INSERT INTO users (name, age) VALUES ('Bob', 25)").await?;

    // Query data
    let results = db.query("SELECT * FROM users WHERE age > 20").await?;

    for row in results {
        let name = row.get("name").unwrap().as_string();
        let age = row.get("age").unwrap().as_int().unwrap();
        println!("User: {} (age: {})", name, age);
    }

    Ok(())
}
```

### Working with Values

```rust
use rust_database_system::prelude::*;

// Create different types of values
let bool_val: DatabaseValue = true.into();
let int_val: DatabaseValue = 42.into();
let long_val: DatabaseValue = 1234567890i64.into();
let double_val: DatabaseValue = 3.14.into();
let string_val: DatabaseValue = "Hello, World!".into();
let bytes_val: DatabaseValue = vec![1, 2, 3, 4].into();

// Convert values
assert_eq!(int_val.as_int(), Some(42));
assert_eq!(int_val.as_long(), Some(42i64));
assert_eq!(string_val.as_string(), "Hello, World!");
assert_eq!(bool_val.as_bool(), Some(true));
```

### Transactions

```rust
use rust_database_system::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut db = SqliteDatabase::new();
    db.connect(":memory:").await?;

    db.execute("CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance REAL)").await?;

    // Begin transaction
    db.begin_transaction().await?;

    match transfer_money(&mut db, 1, 2, 100.0).await {
        Ok(_) => {
            db.commit().await?;
            println!("Transaction committed");
        }
        Err(e) => {
            db.rollback().await?;
            println!("Transaction rolled back: {}", e);
        }
    }

    Ok(())
}

async fn transfer_money(
    db: &mut impl Database,
    from_id: i32,
    to_id: i32,
    amount: f64
) -> Result<()> {
    // Use parameterized queries to prevent SQL injection
    db.execute_with_params(
        "UPDATE accounts SET balance = balance - ? WHERE id = ?",
        &[amount.into(), from_id.into()]
    ).await?;

    db.execute_with_params(
        "UPDATE accounts SET balance = balance + ? WHERE id = ?",
        &[amount.into(), to_id.into()]
    ).await?;

    Ok(())
}
```

### Connection String Builder

```rust
use rust_database_system::prelude::*;

// SQLite
let conn_str = ConnectionBuilder::new(DatabaseType::Sqlite)
    .database("mydb.db")
    .build_connection_string();

// PostgreSQL
let conn_str = ConnectionBuilder::new(DatabaseType::Postgres)
    .host("localhost")
    .port(5432)
    .database("mydb")
    .username("user")
    .password("password")
    .option("sslmode", "require")
    .build_connection_string();

// MySQL
let conn_str = ConnectionBuilder::new(DatabaseType::Mysql)
    .host("localhost")
    .port(3306)
    .database("mydb")
    .username("root")
    .password("password")
    .build_connection_string();
```

## Project Structure

```
rust_database_system/
├── src/
│   ├── core/              # Core types and traits
│   │   ├── database.rs    # Database trait and connection builder
│   │   ├── database_types.rs  # Database type enum
│   │   ├── error.rs       # Error types
│   │   ├── value.rs       # Value types
│   │   └── mod.rs
│   ├── backends/          # Database backend implementations
│   │   ├── sqlite.rs      # SQLite implementation
│   │   └── mod.rs
│   └── lib.rs
├── examples/              # Example programs
│   ├── basic_usage.rs
│   ├── transactions.rs
│   └── async_operations.rs
├── tests/                 # Integration tests
├── benches/              # Benchmarks
├── Cargo.toml
└── README.md
```

## Comparison with C++ Version

| Feature | C++ Version | Rust Version |
|---------|-------------|--------------|
| Type Safety | ✓ (C++20) | ✓ (Rust) |
| Thread Safety | Manual (mutex) | Automatic (Arc+Mutex) |
| Memory Safety | Manual (smart pointers) | Automatic (ownership) |
| Async Support | C++20 coroutines | Tokio async/await |
| Connection Pooling | ✓ | Planned |
| Query Builders | ✓ | Planned |
| ORM Support | ✓ | Planned |
| Performance | High | High |

## Building

### Prerequisites

- Rust 1.70 or later
- Cargo

### Build Commands

```bash
# Build the project
cargo build

# Build with release optimizations
cargo build --release

# Run tests
cargo test

# Run tests with SQLite feature
cargo test --features sqlite

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open
```

## Examples

See the `examples/` directory for more examples:

```bash
# Run the basic example
cargo run --example basic_usage --features sqlite

# Run the transaction example
cargo run --example transactions --features sqlite

# Run the async operations example
cargo run --example async_operations --features sqlite
```

## Features

The library supports feature flags for different database backends:

```toml
[dependencies]
rust_database_system = { version = "0.1", features = ["sqlite"] }

# Or enable multiple backends
rust_database_system = { version = "0.1", features = ["sqlite", "postgres", "mysql"] }

# Or enable all backends
rust_database_system = { version = "0.1", features = ["all-databases"] }
```

Available features:
- `sqlite` - SQLite support (default)
- `postgres` - PostgreSQL support (planned)
- `mysql` - MySQL support (planned)
- `redis_support` - Redis support (planned)
- `mongodb_support` - MongoDB support (planned)
- `all-databases` - All database backends

## Performance

The Rust implementation provides comparable or better performance than the C++ version:

- **Zero-cost abstractions**: No runtime overhead for type safety
- **Memory efficiency**: Efficient use of Arc and Mutex
- **Thread safety**: Safe concurrent access without data races
- **Async performance**: Non-blocking I/O with Tokio runtime

## Security

### SQL Injection Prevention

**⚠️ IMPORTANT**: Always use parameterized queries when working with user input to prevent SQL injection attacks.

**✅ DO** use `execute_with_params` and `query_with_params`:

```rust
// Safe: Using parameterized queries
let username = user_input; // User-provided data
db.execute_with_params(
    "INSERT INTO users (name, age) VALUES (?, ?)",
    &[username.into(), age.into()]
).await?;

db.query_with_params(
    "SELECT * FROM users WHERE name = ?",
    &[username.into()]
).await?;
```

**❌ DON'T** use string concatenation or format!:

```rust
// UNSAFE: Vulnerable to SQL injection!
let username = user_input; // Could be "'; DROP TABLE users; --"
db.execute(&format!(
    "INSERT INTO users (name) VALUES ('{}')",
    username
)).await?; // DON'T DO THIS!
```

### Best Practices

1. **Always parameterize user input**: Use `?` placeholders and pass values as parameters
2. **Validate input data**: Check data types and ranges before database operations
3. **Use transactions**: Group related operations to maintain data integrity
4. **Handle errors properly**: Don't expose database error details to end users
5. **Limit query results**: Use LIMIT clauses to prevent excessive data retrieval
6. **Use prepared statements**: Parameterized queries are automatically prepared and cached

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the BSD 3-Clause License - see the LICENSE file for details.

## Acknowledgments

- Original C++ implementation: [database_system](https://github.com/kcenon/database_system)
- Inspired by: [rust_container_system](https://github.com/kcenon/rust_container_system)
- Built with Rust's excellent ecosystem

## Related Projects

- **database_system**: Original C++ implementation
- **rust_container_system**: Rust container framework
- **messaging_system**: Message persistence backend
- **network_system**: Network transport layer

---

Made with ❤️ in Rust
