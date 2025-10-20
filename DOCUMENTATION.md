# Rust Database System - 종합 문서
# Rust Database System - Comprehensive Documentation

---

## 목차 (Table of Contents)

1. [프로젝트 개요](#1-프로젝트-개요)
2. [아키텍처](#2-아키텍처)
3. [Core 모듈](#3-core-모듈)
   - 3.1 [Error 처리](#31-error-처리)
   - 3.2 [Database Types](#32-database-types)
   - 3.3 [Value Types](#33-value-types)
   - 3.4 [Database Trait](#34-database-trait)
4. [Backends 모듈](#4-backends-모듈)
   - 4.1 [SQLite Backend](#41-sqlite-backend)
5. [학습 가이드](#5-학습-가이드)
6. [사용 예제](#6-사용-예제)

---

## 1. 프로젝트 개요

### Rust Database System

Rust로 구현된 프로덕션급 고성능 데이터베이스 추상화 계층입니다.
여러 데이터베이스 백엔드에 대한 통합된 접근을 제공하며, 연결 풀링, 트랜잭션 관리, 타입 안전한 쿼리 빌딩과 같은 고급 기능을 갖추고 있습니다.

A production-ready, high-performance Rust database abstraction layer designed to provide
unified access to multiple database backends with advanced features including connection
pooling, transaction management, and type-safe query building.

### 주요 특징 (Features)

- **타입 안전성 (Type Safety)**: 컴파일 타임 타입 체크가 있는 강타입 값 시스템
- **스레드 안전성 (Thread Safety)**: parking_lot을 사용한 내장 스레드 안전 작업
- **메모리 효율성 (Memory Efficiency)**: Arc와 스마트 포인터를 사용한 효율적인 메모리 관리
- **비동기 지원 (Async Support)**: Tokio 런타임을 사용한 완전한 async/await 지원
- **트랜잭션 관리 (Transaction Management)**: ACID 준수 트랜잭션 지원
- **다중 백엔드 (Multiple Backends)**: SQLite, PostgreSQL, MySQL, MongoDB, Redis 지원

### 학습 목표 (Learning Objectives)

이 라이브러리를 통해 배울 수 있는 것:

1. **Async/Await**: Tokio를 이용한 비동기 프로그래밍
2. **Trait 시스템**: 다형성과 공통 인터페이스 구현
3. **에러 처리**: Result 타입과 thiserror 사용
4. **스마트 포인터**: Arc, Mutex의 사용법
5. **Feature Flags**: Cargo의 조건부 컴파일

---

## 2. 아키텍처

### 모듈 구조

데이터베이스 시스템은 다음과 같은 모듈로 구성됩니다:

```
rust_database_system/
├── core/                    # 핵심 모듈
│   ├── error.rs            # 에러 타입 정의
│   ├── database_types.rs   # 데이터베이스 타입 열거형
│   ├── value.rs            # 값 타입 정의
│   ├── database.rs         # Database trait 정의
│   └── mod.rs              # Core 모듈 진입점
├── backends/                # 구체적인 백엔드 구현
│   ├── sqlite.rs           # SQLite 구현
│   └── mod.rs              # Backends 모듈 진입점
└── lib.rs                   # 라이브러리 루트
```

### Re-export 패턴

#### pub use의 장점

```rust
// Re-export로 단순화된 경우:
use rust_database_system::prelude::*;

// 또는 개별 import:
use rust_database_system::{Database, DatabaseValue, Result};

// 이렇게 하면 다음처럼 깊은 경로를 쓰지 않아도 됩니다:
// use rust_database_system::core::database::Database;
// use rust_database_system::core::value::DatabaseValue;
```

#### 설계 원칙

- **API 단순화**: 사용자가 깊은 경로를 알 필요 없음
- **유연성 유지**: 내부 구조 변경 시 사용자 코드 영향 최소화
- **명확성**: 가장 자주 사용되는 타입만 re-export

---

## 3. Core 모듈

Core 모듈은 데이터베이스 시스템의 기초를 제공합니다.

### 3.1 Error 처리

#### DatabaseError

시스템의 모든 에러를 표현하는 열거형입니다.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Query execution error: {0}")]
    QueryError(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Transaction error: {0}")]
    TransactionError(String),

    // ... 기타 에러 타입
}
```

#### 에러 처리 패턴

**thiserror 크레이트의 장점:**
- `#[error("...")]`로 에러 메시지 자동 생성
- `#[from]`으로 자동 변환 구현
- Display trait 자동 구현

**사용 예제:**

```rust
async fn connect_to_database(conn_str: &str) -> Result<()> {
    let mut db = SqliteDatabase::new();
    db.connect(conn_str).await?;  // ? 연산자로 자동 에러 전파
    Ok(())
}
```

#### Result 타입

```rust
pub type Result<T> = std::result::Result<T, DatabaseError>;
```

이 타입 별칭은 모든 데이터베이스 작업에서 일관된 에러 처리를 제공합니다.

---

### 3.2 Database Types

#### DatabaseType Enum

지원하는 데이터베이스 타입을 정의하는 열거형입니다.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DatabaseType {
    None = 0,        // 지정되지 않음
    Postgres = 1,    // PostgreSQL
    Mysql = 2,       // MySQL/MariaDB
    Sqlite = 3,      // SQLite
    Oracle = 4,      // Oracle (미래 구현)
    Mongodb = 5,     // MongoDB
    Redis = 6,       // Redis
}
```

#### 주요 메서드

**1. to_str() - 문자열로 변환**

```rust
let db_type = DatabaseType::Sqlite;
assert_eq!(db_type.to_str(), "sqlite");
```

**2. from_str() - 문자열에서 변환**

```rust
let db_type = DatabaseType::from_str("postgres");
assert_eq!(db_type, Some(DatabaseType::Postgres));

let db_type = DatabaseType::from_str("postgresql");
assert_eq!(db_type, Some(DatabaseType::Postgres));  // 별칭 지원
```

**3. is_sql() - SQL 데이터베이스 체크**

```rust
assert!(DatabaseType::Postgres.is_sql());
assert!(DatabaseType::Mysql.is_sql());
assert!(!DatabaseType::Redis.is_sql());
```

**4. is_nosql() - NoSQL 데이터베이스 체크**

```rust
assert!(DatabaseType::Mongodb.is_nosql());
assert!(DatabaseType::Redis.is_nosql());
assert!(!DatabaseType::Postgres.is_nosql());
```

**5. supports_transactions() - 트랜잭션 지원 체크**

```rust
assert!(DatabaseType::Sqlite.supports_transactions());
assert!(!DatabaseType::Redis.supports_transactions());
```

**6. supports_async() - 비동기 지원 체크**

```rust
assert!(DatabaseType::Postgres.supports_async());
assert!(!DatabaseType::Sqlite.supports_async());  // SQLite는 동기식
```

#### 데이터베이스 타입 비교표

| Database Type | SQL | NoSQL | Transactions | Async |
|--------------|-----|-------|--------------|-------|
| Postgres | ✓ | - | ✓ | ✓ |
| MySQL | ✓ | - | ✓ | ✓ |
| SQLite | ✓ | - | ✓ | - |
| MongoDB | - | ✓ | ✓ | ✓ |
| Redis | - | ✓ | - | ✓ |

---

### 3.3 Value Types

#### DatabaseValue Enum

데이터베이스에서 읽거나 쓸 수 있는 값의 타입을 정의합니다.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseValue {
    Null,                // NULL 값
    Bool(bool),          // Boolean
    Int(i32),            // 32비트 정수
    Long(i64),           // 64비트 정수
    Float(f32),          // 32비트 부동소수점
    Double(f64),         // 64비트 부동소수점
    String(String),      // UTF-8 문자열
    Bytes(Vec<u8>),      // 바이너리 데이터
    Timestamp(i64),      // 타임스탬프 (마이크로초)
}
```

#### 타입 변환 메서드

**1. as_bool() - Boolean으로 변환**

```rust
let val = DatabaseValue::Bool(true);
assert_eq!(val.as_bool(), Some(true));

let val = DatabaseValue::Int(1);
assert_eq!(val.as_bool(), Some(true));

let val = DatabaseValue::String("true".to_string());
assert_eq!(val.as_bool(), Some(true));
```

**2. as_int() - i32로 변환**

```rust
let val = DatabaseValue::Int(42);
assert_eq!(val.as_int(), Some(42));

let val = DatabaseValue::Long(100);
assert_eq!(val.as_int(), Some(100));

let val = DatabaseValue::String("123".to_string());
assert_eq!(val.as_int(), Some(123));
```

**3. as_long() - i64로 변환**

```rust
let val = DatabaseValue::Long(1234567890);
assert_eq!(val.as_long(), Some(1234567890));

let val = DatabaseValue::Int(42);
assert_eq!(val.as_long(), Some(42));  // 안전한 확장
```

**4. as_double() - f64로 변환**

```rust
let val = DatabaseValue::Double(3.14159);
assert_eq!(val.as_double(), Some(3.14159));

let val = DatabaseValue::Int(42);
assert_eq!(val.as_double(), Some(42.0));
```

**5. as_string() - String으로 변환**

```rust
let val = DatabaseValue::String("Hello".to_string());
assert_eq!(val.as_string(), "Hello");

let val = DatabaseValue::Int(42);
assert_eq!(val.as_string(), "42");

let val = DatabaseValue::Null;
assert_eq!(val.as_string(), "null");
```

**6. as_bytes() - 바이트로 변환**

```rust
let val = DatabaseValue::Bytes(vec![1, 2, 3]);
assert_eq!(val.as_bytes(), Some(&[1u8, 2, 3][..]));

let val = DatabaseValue::String("ABC".to_string());
assert_eq!(val.as_bytes(), Some(&[65u8, 66, 67][..]));  // ASCII codes
```

#### From trait 구현

Rust 타입에서 DatabaseValue로 자동 변환:

```rust
// Boolean
let val: DatabaseValue = true.into();

// Integer
let val: DatabaseValue = 42.into();
let val: DatabaseValue = 42i64.into();

// Float
let val: DatabaseValue = 3.14f32.into();
let val: DatabaseValue = 3.14f64.into();

// String
let val: DatabaseValue = "hello".into();
let val: DatabaseValue = String::from("hello").into();

// Bytes
let val: DatabaseValue = vec![1u8, 2, 3].into();

// Option (None -> Null)
let val: DatabaseValue = Option::<i32>::None.into();
assert_eq!(val, DatabaseValue::Null);

let val: DatabaseValue = Some(42).into();
assert_eq!(val, DatabaseValue::Int(42));
```

#### DatabaseRow와 DatabaseResult

```rust
// 단일 행: 컬럼 이름 -> 값 매핑
pub type DatabaseRow = HashMap<String, DatabaseValue>;

// 쿼리 결과: 여러 행의 배열
pub type DatabaseResult = Vec<DatabaseRow>;
```

**사용 예제:**

```rust
let results: DatabaseResult = db.query("SELECT * FROM users").await?;

for row in results {
    // 컬럼 값 접근
    if let Some(name) = row.get("name") {
        println!("Name: {}", name.as_string());
    }

    if let Some(age) = row.get("age") {
        if let Some(age_num) = age.as_int() {
            println!("Age: {}", age_num);
        }
    }
}
```

---

### 3.4 Database Trait

#### Database Trait 정의

모든 데이터베이스 백엔드가 구현해야 하는 공통 인터페이스입니다.

```rust
#[async_trait]
pub trait Database: Send + Sync {
    // 데이터베이스 타입 반환
    fn database_type(&self) -> DatabaseType;

    // 연결 관리
    async fn connect(&mut self, connection_string: &str) -> Result<()>;
    fn is_connected(&self) -> bool;
    async fn disconnect(&mut self) -> Result<()>;

    // 쿼리 실행
    async fn execute(&mut self, query: &str) -> Result<u64>;
    async fn query(&mut self, query: &str) -> Result<DatabaseResult>;
    async fn query_with_params(&mut self, query: &str, params: &[DatabaseValue])
        -> Result<DatabaseResult>;

    // 트랜잭션
    async fn begin_transaction(&mut self) -> Result<()>;
    async fn commit(&mut self) -> Result<()>;
    async fn rollback(&mut self) -> Result<()>;
    fn in_transaction(&self) -> bool;
}
```

#### 메서드 설명

**1. database_type() - 데이터베이스 타입 식별**

```rust
let db = SqliteDatabase::new();
assert_eq!(db.database_type(), DatabaseType::Sqlite);
```

**2. connect() - 데이터베이스 연결**

```rust
let mut db = SqliteDatabase::new();
db.connect(":memory:").await?;  // 메모리 DB
db.connect("mydb.db").await?;   // 파일 DB
```

**3. execute() - 쿼리 실행 (INSERT, UPDATE, DELETE)**

```rust
// CREATE TABLE
db.execute("CREATE TABLE users (id INTEGER, name TEXT)").await?;

// INSERT - 영향받은 행 수 반환
let affected = db.execute("INSERT INTO users VALUES (1, 'Alice')").await?;
assert_eq!(affected, 1);

// UPDATE
let affected = db.execute("UPDATE users SET name = 'Bob' WHERE id = 1").await?;
assert_eq!(affected, 1);

// DELETE
let affected = db.execute("DELETE FROM users WHERE id = 1").await?;
assert_eq!(affected, 1);
```

**4. query() - SELECT 쿼리 실행**

```rust
let results = db.query("SELECT * FROM users").await?;

for row in results {
    let id = row.get("id").unwrap().as_int().unwrap();
    let name = row.get("name").unwrap().as_string();
    println!("User {}: {}", id, name);
}
```

**5. query_with_params() - 파라미터화된 쿼리 (준비된 구문)**

```rust
let params = vec![
    DatabaseValue::from("Alice"),
    DatabaseValue::from(30),
];

let results = db.query_with_params(
    "SELECT * FROM users WHERE name = ? AND age > ?",
    &params
).await?;
```

**6. 트랜잭션 메서드**

```rust
// 트랜잭션 시작
db.begin_transaction().await?;
assert!(db.in_transaction());

// 작업 수행
db.execute("INSERT INTO accounts VALUES (1, 1000.0)").await?;
db.execute("UPDATE accounts SET balance = balance - 100 WHERE id = 1").await?;

// 커밋 또는 롤백
if success {
    db.commit().await?;
} else {
    db.rollback().await?;
}

assert!(!db.in_transaction());
```

#### ConnectionBuilder

연결 문자열을 쉽게 생성하는 빌더 패턴:

```rust
pub struct ConnectionBuilder {
    db_type: DatabaseType,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    username: Option<String>,
    password: Option<String>,
    options: HashMap<String, String>,
}
```

**사용 예제:**

```rust
// SQLite
let conn_str = ConnectionBuilder::new(DatabaseType::Sqlite)
    .database("mydb.db")
    .build_connection_string();
// 결과: "mydb.db"

// PostgreSQL
let conn_str = ConnectionBuilder::new(DatabaseType::Postgres)
    .host("localhost")
    .port(5432)
    .database("mydb")
    .username("user")
    .password("pass")
    .option("sslmode", "require")
    .build_connection_string();
// 결과: "host=localhost port=5432 dbname=mydb user=user password=pass sslmode=require"

// MySQL
let conn_str = ConnectionBuilder::new(DatabaseType::Mysql)
    .host("localhost")
    .port(3306)
    .database("mydb")
    .username("root")
    .password("pass")
    .build_connection_string();
// 결과: "mysql://root:pass@localhost:3306/mydb"
```

---

## 4. Backends 모듈

Backend 모듈은 Database trait의 구체적인 구현을 제공합니다.

### 4.1 SQLite Backend

#### SqliteDatabase 구조

```rust
pub struct SqliteDatabase {
    connection: Arc<Mutex<Option<Connection>>>,
    in_transaction: Arc<Mutex<bool>>,
}
```

**설계 선택:**
- **Arc<Mutex<...>>**: 스레드 안전한 공유 소유권
- **Option<Connection>**: 연결되지 않은 상태 표현
- **parking_lot::Mutex**: 표준 라이브러리보다 빠른 뮤텍스

#### 주요 메서드 구현

**1. connect() - SQLite 연결**

```rust
async fn connect(&mut self, connection_string: &str) -> Result<()> {
    let conn = Connection::open(connection_string)?;

    // Foreign key 활성화
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    let mut connection = self.connection.lock();
    *connection = Some(conn);

    Ok(())
}
```

**연결 문자열 예제:**
```rust
// 메모리 DB
db.connect(":memory:").await?;

// 파일 DB
db.connect("mydb.db").await?;
db.connect("/path/to/database.db").await?;

// 읽기 전용
db.connect("file:mydb.db?mode=ro").await?;
```

**2. execute() - 쿼리 실행**

```rust
async fn execute(&mut self, query: &str) -> Result<u64> {
    let connection = self.connection.lock();
    let conn = connection
        .as_ref()
        .ok_or_else(|| DatabaseError::connection("Not connected"))?;

    let affected = conn.execute(query, [])?;
    Ok(affected as u64)
}
```

**3. query() - SELECT 쿼리**

```rust
async fn query(&mut self, query: &str) -> Result<DatabaseResult> {
    let connection = self.connection.lock();
    let conn = connection
        .as_ref()
        .ok_or_else(|| DatabaseError::connection("Not connected"))?;

    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map([], Self::row_to_database_row)?;

    let mut results = Vec::new();
    for row_result in rows {
        results.push(row_result?);
    }

    Ok(results)
}
```

**4. row_to_database_row() - SQLite Row 변환**

```rust
fn row_to_database_row(row: &Row) -> rusqlite::Result<DatabaseRow> {
    let mut db_row = DatabaseRow::new();
    let column_count = row.as_ref().column_count();

    for i in 0..column_count {
        let column_name = row.as_ref().column_name(i)?.to_string();
        let value = match row.get_ref(i)? {
            ValueRef::Null => DatabaseValue::Null,
            ValueRef::Integer(v) => DatabaseValue::Long(v),
            ValueRef::Real(v) => DatabaseValue::Double(v),
            ValueRef::Text(v) => DatabaseValue::String(
                String::from_utf8_lossy(v).to_string()
            ),
            ValueRef::Blob(v) => DatabaseValue::Bytes(v.to_vec()),
        };
        db_row.insert(column_name, value);
    }

    Ok(db_row)
}
```

#### 트랜잭션 구현

**1. begin_transaction()**

```rust
async fn begin_transaction(&mut self) -> Result<()> {
    let connection = self.connection.lock();
    let conn = connection
        .as_ref()
        .ok_or_else(|| DatabaseError::connection("Not connected"))?;

    conn.execute("BEGIN TRANSACTION", [])?;

    let mut in_transaction = self.in_transaction.lock();
    *in_transaction = true;

    Ok(())
}
```

**2. commit()**

```rust
async fn commit(&mut self) -> Result<()> {
    let connection = self.connection.lock();
    let conn = connection
        .as_ref()
        .ok_or_else(|| DatabaseError::connection("Not connected"))?;

    conn.execute("COMMIT", [])?;

    let mut in_transaction = self.in_transaction.lock();
    *in_transaction = false;

    Ok(())
}
```

**3. rollback()**

```rust
async fn rollback(&mut self) -> Result<()> {
    let connection = self.connection.lock();
    let conn = connection
        .as_ref()
        .ok_or_else(|| DatabaseError::connection("Not connected"))?;

    conn.execute("ROLLBACK", [])?;

    let mut in_transaction = self.in_transaction.lock();
    *in_transaction = false;

    Ok(())
}
```

#### SQLite 특징

**장점:**
- **Zero-configuration**: 설치나 설정 불필요
- **Self-contained**: 단일 파일 데이터베이스
- **Cross-platform**: 모든 플랫폼에서 동일하게 동작
- **ACID**: 완전한 트랜잭션 지원
- **Fast**: 로컬 파일 액세스로 빠른 속도

**제한사항:**
- **Concurrency**: 동시 쓰기 제한 (하나의 writer)
- **Network**: 네트워크 접근 불가 (로컬 파일만)
- **Size**: 대용량 데이터에는 부적합 (일반적으로 < 1TB)

**사용 사례:**
- 모바일 앱 데이터
- 로컬 캐시
- 임베디드 시스템
- 테스트 환경
- 중소 규모 웹 앱

---

## 5. 학습 가이드

### 5.1 Async/Await

#### Async 함수

```rust
async fn fetch_users(db: &mut impl Database) -> Result<Vec<String>> {
    let results = db.query("SELECT name FROM users").await?;

    let names = results
        .iter()
        .filter_map(|row| row.get("name"))
        .map(|val| val.as_string())
        .collect();

    Ok(names)
}
```

#### Tokio 런타임

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mut db = SqliteDatabase::new();
    db.connect(":memory:").await?;

    let users = fetch_users(&mut db).await?;
    println!("Users: {:?}", users);

    Ok(())
}
```

#### 병렬 작업

```rust
use tokio::join;

async fn fetch_all_data(db: &mut impl Database) -> Result<()> {
    // 여러 쿼리를 병렬로 실행
    let (users, posts, comments) = join!(
        db.query("SELECT * FROM users"),
        db.query("SELECT * FROM posts"),
        db.query("SELECT * FROM comments")
    );

    let users = users?;
    let posts = posts?;
    let comments = comments?;

    println!("Loaded {} users, {} posts, {} comments",
        users.len(), posts.len(), comments.len());

    Ok(())
}
```

### 5.2 Trait 시스템

#### Database Trait 구현

```rust
struct MyCustomDatabase {
    // ...
}

#[async_trait]
impl Database for MyCustomDatabase {
    fn database_type(&self) -> DatabaseType {
        DatabaseType::None
    }

    async fn connect(&mut self, connection_string: &str) -> Result<()> {
        // 구현
        Ok(())
    }

    // ... 나머지 메서드 구현
}
```

#### Trait 객체 사용

```rust
async fn process_database(db: &mut dyn Database) -> Result<()> {
    println!("Using database: {}", db.database_type());
    db.execute("CREATE TABLE test (id INTEGER)").await?;
    Ok(())
}

// 사용
let mut sqlite_db = SqliteDatabase::new();
process_database(&mut sqlite_db).await?;
```

### 5.3 에러 처리

#### Result와 ? 연산자

```rust
async fn create_user(db: &mut impl Database, name: &str) -> Result<()> {
    // ? 연산자로 에러 자동 전파
    db.execute(&format!(
        "INSERT INTO users (name) VALUES ('{}')", name
    )).await?;

    Ok(())
}
```

#### 커스텀 에러 생성

```rust
fn validate_age(age: i32) -> Result<()> {
    if age < 0 {
        return Err(DatabaseError::query("Age cannot be negative"));
    }
    if age > 150 {
        return Err(DatabaseError::query("Age is unrealistic"));
    }
    Ok(())
}
```

#### 에러 처리 패턴

```rust
async fn safe_query(db: &mut impl Database, query: &str) -> Result<DatabaseResult> {
    match db.query(query).await {
        Ok(results) => {
            println!("Query successful: {} rows", results.len());
            Ok(results)
        }
        Err(e) => {
            eprintln!("Query failed: {}", e);
            Err(e)
        }
    }
}
```

---

## 6. 사용 예제

### 6.1 기본 CRUD 작업

```rust
use rust_database_system::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut db = SqliteDatabase::new();
    db.connect(":memory:").await?;

    // CREATE
    db.execute(
        "CREATE TABLE products (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            price REAL NOT NULL,
            stock INTEGER DEFAULT 0
        )"
    ).await?;

    // INSERT
    db.execute(
        "INSERT INTO products (name, price, stock)
         VALUES ('Laptop', 999.99, 10)"
    ).await?;

    db.execute(
        "INSERT INTO products (name, price, stock)
         VALUES ('Mouse', 29.99, 50)"
    ).await?;

    // SELECT
    let results = db.query("SELECT * FROM products").await?;
    for row in results {
        let name = row.get("name").unwrap().as_string();
        let price = row.get("price").unwrap().as_double().unwrap();
        println!("{}: ${:.2}", name, price);
    }

    // UPDATE
    db.execute("UPDATE products SET price = 899.99 WHERE name = 'Laptop'")
        .await?;

    // DELETE
    db.execute("DELETE FROM products WHERE stock = 0").await?;

    Ok(())
}
```

### 6.2 트랜잭션을 이용한 은행 이체

```rust
async fn transfer_money(
    db: &mut impl Database,
    from_account: i32,
    to_account: i32,
    amount: f64
) -> Result<()> {
    // 트랜잭션 시작
    db.begin_transaction().await?;

    // 출금
    let affected = db.execute(&format!(
        "UPDATE accounts SET balance = balance - {}
         WHERE id = {} AND balance >= {}",
        amount, from_account, amount
    )).await?;

    if affected == 0 {
        db.rollback().await?;
        return Err(DatabaseError::query("Insufficient funds"));
    }

    // 입금
    db.execute(&format!(
        "UPDATE accounts SET balance = balance + {} WHERE id = {}",
        amount, to_account
    )).await?;

    // 커밋
    db.commit().await?;

    Ok(())
}
```

### 6.3 배치 삽입

```rust
async fn batch_insert_users(
    db: &mut impl Database,
    users: &[(String, i32)]
) -> Result<()> {
    db.begin_transaction().await?;

    for (name, age) in users {
        db.execute(&format!(
            "INSERT INTO users (name, age) VALUES ('{}', {})",
            name, age
        )).await?;
    }

    db.commit().await?;
    Ok(())
}

// 사용
let users = vec![
    ("Alice".to_string(), 30),
    ("Bob".to_string(), 25),
    ("Charlie".to_string(), 35),
];

batch_insert_users(&mut db, &users).await?;
```

### 6.4 조건부 쿼리

```rust
async fn find_users(
    db: &mut impl Database,
    min_age: Option<i32>,
    max_age: Option<i32>,
    name_pattern: Option<&str>
) -> Result<DatabaseResult> {
    let mut query = "SELECT * FROM users WHERE 1=1".to_string();

    if let Some(min) = min_age {
        query.push_str(&format!(" AND age >= {}", min));
    }

    if let Some(max) = max_age {
        query.push_str(&format!(" AND age <= {}", max));
    }

    if let Some(pattern) = name_pattern {
        query.push_str(&format!(" AND name LIKE '%{}%'", pattern));
    }

    db.query(&query).await
}

// 사용
let results = find_users(&mut db, Some(20), Some(40), Some("Al")).await?;
```

---

## 부록 A: 에러 코드 참조

| 에러 타입 | 설명 | 발생 상황 |
|----------|------|-----------|
| ConnectionError | 연결 에러 | 데이터베이스 연결 실패 |
| QueryError | 쿼리 실행 에러 | SQL 문법 오류, 실행 실패 |
| TypeMismatch | 타입 불일치 | 잘못된 타입 변환 시도 |
| TransactionError | 트랜잭션 에러 | 트랜잭션 커밋/롤백 실패 |
| DatabaseNotFound | 데이터베이스 없음 | 지정한 데이터베이스가 존재하지 않음 |
| TableNotFound | 테이블 없음 | 지정한 테이블이 존재하지 않음 |

---

## 부록 B: 지원하는 SQL 구문 (SQLite)

### DDL (Data Definition Language)

```sql
-- CREATE TABLE
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    age INTEGER CHECK(age >= 0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- ALTER TABLE
ALTER TABLE users ADD COLUMN email TEXT;

-- DROP TABLE
DROP TABLE IF EXISTS users;

-- CREATE INDEX
CREATE INDEX idx_users_name ON users(name);
```

### DML (Data Manipulation Language)

```sql
-- INSERT
INSERT INTO users (name, age) VALUES ('Alice', 30);
INSERT INTO users (name, age) VALUES ('Bob', 25), ('Charlie', 35);

-- UPDATE
UPDATE users SET age = 31 WHERE name = 'Alice';
UPDATE users SET age = age + 1;

-- DELETE
DELETE FROM users WHERE age < 18;
DELETE FROM users;  -- 모든 행 삭제

-- SELECT
SELECT * FROM users;
SELECT name, age FROM users WHERE age > 25;
SELECT COUNT(*) FROM users;
SELECT AVG(age) FROM users;
```

### 고급 쿼리

```sql
-- JOIN
SELECT users.name, orders.total
FROM users
INNER JOIN orders ON users.id = orders.user_id;

-- GROUP BY
SELECT age, COUNT(*) as count
FROM users
GROUP BY age
HAVING count > 1;

-- ORDER BY
SELECT * FROM users ORDER BY age DESC, name ASC;

-- LIMIT/OFFSET
SELECT * FROM users LIMIT 10 OFFSET 20;

-- LIKE
SELECT * FROM users WHERE name LIKE 'A%';

-- IN
SELECT * FROM users WHERE age IN (25, 30, 35);
```

---

## 변경 이력

### Version 0.1.0 (2025-01-XX)
- 초기 릴리스
- SQLite 백엔드 구현
- Core 모듈 (error, types, value, database)
- 트랜잭션 지원
- 비동기 API
- 종합 문서 작성

---

**문서 작성일:** 2025년 1월
**작성자:** Database System Team
**라이선스:** BSD 3-Clause

이 문서는 Rust Database System의 모든 주석과 문서를 통합하여 작성되었습니다.
초보자부터 중급자까지 학습할 수 있도록 상세한 설명과 예제를 포함하고 있습니다.
