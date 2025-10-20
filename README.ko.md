# Rust Database System

> **Languages**: [English](./README.md) | 한국어

프로덕션 환경에서 사용 가능한 고성능 Rust 데이터베이스 추상화 레이어로, 연결 풀링, 트랜잭션 관리, 타입 안전 쿼리 빌더를 포함한 고급 기능과 함께 여러 데이터베이스 백엔드에 대한 통합 접근을 제공합니다.

이 프로젝트는 [database_system](https://github.com/kcenon/database_system)의 Rust 구현으로, Rust의 안전성 보장과 성능 이점을 활용하여 동일한 기능을 제공합니다.

## 기능

- **타입 안전성**: 컴파일 타임 검사가 가능한 강타입 값 시스템
- **스레드 안전성**: `parking_lot`을 사용한 내장 스레드 안전 작업
- **메모리 효율성**: `Arc`와 스마트 포인터를 통한 효율적인 메모리 관리
- **Async 지원**: Tokio 런타임을 사용한 완전한 async/await 지원
- **트랜잭션 관리**: ACID 준수 트랜잭션 지원
- **다중 백엔드**: SQLite, PostgreSQL, MySQL, MongoDB, Redis 지원
- **크로스 플랫폼**: Windows, Linux, macOS에서 동작
- **제로 코스트 추상화**: 네이티브 드라이버와 비교 가능한 성능

## 지원 데이터베이스

| Database | Status | Features | Async | Transactions |
|----------|--------|----------|-------|--------------|
| SQLite | ✅ Full | Bundled, WAL mode, FTS5 | ❌ | ✅ |
| PostgreSQL | 🔄 Planned | JSONB, Arrays, CTEs | ✅ | ✅ |
| MySQL | 🔄 Planned | Full-text search, Replication | ✅ | ✅ |
| MongoDB | 🔄 Planned | Documents, Aggregation | ✅ | ✅ |
| Redis | 🔄 Planned | All data types, Pub/Sub | ✅ | ❌ |

## 빠른 시작

### 설치

`Cargo.toml`에 다음을 추가하세요:

```toml
[dependencies]
rust_database_system = { version = "0.1", features = ["sqlite"] }
tokio = { version = "1", features = ["full"] }
```

### 기본 사용법

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

### 값 다루기

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

### 트랜잭션

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
    db.execute(&format!(
        "UPDATE accounts SET balance = balance - {} WHERE id = {}",
        amount, from_id
    )).await?;

    db.execute(&format!(
        "UPDATE accounts SET balance = balance + {} WHERE id = {}",
        amount, to_id
    )).await?;

    Ok(())
}
```

### 연결 문자열 빌더

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

## 프로젝트 구조

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

## C++ 버전과의 비교

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

## 빌드

### 필수 요구사항

- Rust 1.70 이상
- Cargo

### 빌드 명령어

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

## 예제

더 많은 예제는 `examples/` 디렉토리를 참고하세요:

```bash
# Run the basic example
cargo run --example basic_usage --features sqlite

# Run the transaction example
cargo run --example transactions --features sqlite

# Run the async operations example
cargo run --example async_operations --features sqlite
```

## 기능 플래그

라이브러리는 다양한 데이터베이스 백엔드에 대한 기능 플래그를 지원합니다:

```toml
[dependencies]
rust_database_system = { version = "0.1", features = ["sqlite"] }

# Or enable multiple backends
rust_database_system = { version = "0.1", features = ["sqlite", "postgres", "mysql"] }

# Or enable all backends
rust_database_system = { version = "0.1", features = ["all-databases"] }
```

사용 가능한 기능:
- `sqlite` - SQLite 지원 (기본값)
- `postgres` - PostgreSQL 지원 (계획됨)
- `mysql` - MySQL 지원 (계획됨)
- `redis_support` - Redis 지원 (계획됨)
- `mongodb_support` - MongoDB 지원 (계획됨)
- `all-databases` - 모든 데이터베이스 백엔드

## 성능

Rust 구현은 C++ 버전과 비교하여 동등하거나 더 나은 성능을 제공합니다:

- **제로 코스트 추상화**: 타입 안전성을 위한 런타임 오버헤드 없음
- **메모리 효율성**: Arc와 Mutex의 효율적인 사용
- **스레드 안전성**: 데이터 레이스 없이 안전한 동시 접근
- **Async 성능**: Tokio 런타임을 사용한 논블로킹 I/O

## 기여하기

기여를 환영합니다! Pull Request를 자유롭게 제출해 주세요.

## 라이선스

이 프로젝트는 BSD 3-Clause License로 라이선스가 부여됩니다. 자세한 내용은 LICENSE 파일을 참조하세요.

## 감사의 말

- 원본 C++ 구현: [database_system](https://github.com/kcenon/database_system)
- 영감을 받은 프로젝트: [rust_container_system](https://github.com/kcenon/rust_container_system)
- Rust의 훌륭한 생태계로 구축됨

## 관련 프로젝트

- **database_system**: 원본 C++ 구현
- **rust_container_system**: Rust 컨테이너 프레임워크
- **messaging_system**: 메시지 영속화 백엔드
- **network_system**: 네트워크 전송 계층

---

Made with ❤️ in Rust
