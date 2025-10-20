# Rust Database System 아키텍처

> **Languages**: [English](./ARCHITECTURE.md) | 한국어

## 개요

Rust Database System은 여러 데이터베이스 백엔드에 대한 통합 추상화 레이어를 제공하며, 타입 안전 작업, 트랜잭션 관리, async/await 지원을 제공합니다.

## 아키텍처 다이어그램

```
┌─────────────────────────────────────────────────────────┐
│                  애플리케이션 레이어                      │
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
    │ 백엔드   │ │ 백엔드   │ │ 백엔드   │
    └─────────┘ └─────────┘ └─────────┘
```

## 핵심 구성요소

### 1. Database Trait (`core/database.rs`)

모든 데이터베이스 작업을 정의하는 핵심 추상화:

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

**설계 결정사항:**
- `async_trait`로 async/await 지원
- `Send + Sync` 바운드로 스레드 안전성 보장
- 가변 참조로 상태 관리 가능 (연결, 트랜잭션)
- Result 타입으로 포괄적인 에러 처리

### 2. Value 시스템 (`core/value.rs`)

다양한 데이터베이스 간 타입 안전 값 표현:

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

**특징:**
- `From` trait 구현으로 타입 변환
- 안전한 추출 메서드 (`as_int()`, `as_string()` 등)
- Null 처리
- 크로스 데이터베이스 타입 매핑

### 3. 에러 처리 (`core/error.rs`)

`thiserror`를 사용한 포괄적인 에러 타입:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("연결 오류: {0}")]
    ConnectionError(String),

    #[error("쿼리 실행 오류: {0}")]
    QueryError(String),

    #[error("타입 불일치: 예상 {expected}, 실제 {actual}")]
    TypeMismatch { expected: String, actual: String },

    // 백엔드별 에러
    #[cfg(feature = "sqlite")]
    #[error("SQLite 오류: {0}")]
    SqliteError(#[from] rusqlite::Error),
}
```

### 4. 백엔드 구현 (`backends/`)

각 백엔드는 `Database` trait을 구현합니다:

#### SQLite 백엔드 (`backends/sqlite.rs`)

```rust
pub struct SqliteDatabase {
    connection: Arc<Mutex<Option<Connection>>>,
    in_transaction: Arc<Mutex<bool>>,
}
```

**스레드 안전성:**
- `Arc<Mutex<>>` 패턴으로 공유 연결
- 별도의 트랜잭션 상태 추적
- 성능 향상을 위한 `parking_lot::Mutex`

**주요 기능:**
- 번들된 SQLite (외부 의존성 없음)
- 기본적으로 외래키 지원 활성화
- Prepared statement 지원

## 데이터 플로우

### 쿼리 실행 플로우

```
애플리케이션
    │
    ├─> db.query("SELECT * FROM users")
    │
    ▼
Database Trait
    │
    ├─> 백엔드별 구현
    │
    ▼
SQLite 백엔드
    │
    ├─> rusqlite로 쿼리 파싱
    ├─> 쿼리 실행
    ├─> rusqlite::Row를 DatabaseRow로 변환
    │
    ▼
DatabaseResult (Vec<DatabaseRow>)
    │
    ▼
애플리케이션이 결과 수신
```

### 트랜잭션 플로우

```
1. begin_transaction()
   │
   ├─> "BEGIN TRANSACTION" 실행
   ├─> in_transaction = true 설정
   │
2. execute() 작업들
   │
   ├─> 트랜잭션 내에서 작업 수행
   │
3a. commit()                    3b. rollback()
    │                               │
    ├─> "COMMIT" 실행               ├─> "ROLLBACK" 실행
    ├─> in_transaction=false 설정   ├─> in_transaction=false 설정
```

## 연결 관리

### Connection Builder 패턴

```rust
let conn_str = ConnectionBuilder::new(DatabaseType::Postgres)
    .host("localhost")
    .port(5432)
    .database("mydb")
    .username("user")
    .password("pass")
    .build_connection_string();
```

**장점:**
- 타입 안전 연결 문자열 생성
- 데이터베이스별 포맷 처리
- 연결 문자열 오류 감소

### 향후: Connection Pooling

연결 풀링 계획 아키텍처:

```rust
pub trait ConnectionPool: Send + Sync {
    async fn acquire(&self) -> Result<Arc<dyn Database>>;
    async fn release(&self, connection: Arc<dyn Database>) -> Result<()>;
    fn size(&self) -> usize;
    fn active_count(&self) -> usize;
}
```

## 타입 매핑

### Rust ↔ Database 타입 매핑

| Rust 타입 | SQLite 타입 | PostgreSQL 타입 | MySQL 타입 |
|-----------|-------------|-----------------|------------|
| `bool` | `INTEGER` | `BOOLEAN` | `TINYINT(1)` |
| `i32` | `INTEGER` | `INT` | `INT` |
| `i64` | `INTEGER` | `BIGINT` | `BIGINT` |
| `f32` | `REAL` | `REAL` | `FLOAT` |
| `f64` | `REAL` | `DOUBLE PRECISION` | `DOUBLE` |
| `String` | `TEXT` | `VARCHAR/TEXT` | `VARCHAR/TEXT` |
| `Vec<u8>` | `BLOB` | `BYTEA` | `BLOB` |
| `Option<T>` | `NULL` | `NULL` | `NULL` |

## 에러 처리 전략

### 에러 전파

```rust
pub type Result<T> = std::result::Result<T, DatabaseError>;

// 애플리케이션 코드
async fn example() -> Result<()> {
    let mut db = SqliteDatabase::new();
    db.connect(":memory:").await?;  // rusqlite::Error 자동 변환
    db.execute("CREATE TABLE users (id INTEGER)").await?;
    Ok(())
}
```

### 에러 컨텍스트

에러에는 디버깅을 위한 컨텍스트가 포함됩니다:
- 연결 에러: 연결 문자열 포함 (민감정보 제거)
- 쿼리 에러: 쿼리 텍스트 포함 (긴 경우 잘림)
- 타입 에러: 예상 타입과 실제 타입 포함

## 테스트 전략

### 단위 테스트

각 컴포넌트에 단위 테스트:
- Value 변환
- Error 생성
- 연결 문자열 빌드

### 통합 테스트

백엔드별 테스트:
- 연결 생명주기
- 쿼리 실행
- 트랜잭션 원자성
- 타입 변환 왕복

### 테스트 예제

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
    assert_eq!(results.len(), 0); // 롤백 성공
}
```

## 성능 고려사항

### 비동기 작업

- 모든 I/O 작업은 비동기
- Tokio 런타임으로 비동기 실행
- 논블로킹 데이터베이스 작업

### 메모리 관리

- `Arc`로 복제 없이 공유 소유권
- `Mutex`는 실제 I/O 중에만 잠금
- 값에 `&str` 대신 `String` 사용으로 효율적인 문자열 처리

### 쿼리 최적화

- Prepared statement 지원 (파싱 오버헤드 감소)
- 향후 배치 작업 계획
- 향후 연결 풀링 계획

## 향후 개선사항

### 계획된 기능

1. **Connection Pooling**
   - 연결 재사용
   - 설정 가능한 풀 크기
   - 연결 상태 체크

2. **Query Builder**
   ```rust
   db.select()
       .from("users")
       .where_eq("status", "active")
       .execute()
       .await?;
   ```

3. **ORM 기능**
   - Struct-to-table 매핑
   - 자동 마이그레이션
   - 관계 지원

4. **추가 백엔드**
   - PostgreSQL (계획)
   - MySQL (계획)
   - MongoDB (계획)
   - Redis (계획)

## 설계 철학

### 핵심 원칙

1. **안전성 우선**: Rust의 타입 시스템을 사용하여 컴파일 타임에 에러 방지
2. **간단한 API**: 일반적인 작업은 간단하고 직관적이어야 함
3. **백엔드 독립적**: 한 번 작성하면 지원되는 모든 데이터베이스에서 실행
4. **성능**: 가능한 경우 제로 코스트 추상화
5. **비동기 네이티브**: 현대적인 비동기 Rust 애플리케이션을 위해 구축

### 트레이드오프

- **유연성 vs. 단순성**: 최대 유연성보다 단순한 API 선택
- **타입 안전성 vs. 동적 쿼리**: 런타임 유연성보다 컴파일 타임 안전성 선택
- **성능 vs. 사용성**: 더 나은 API를 위해 약간 더 많은 할당 허용

## 통합 예제

### 웹 프레임워크와 통합

```rust
// Axum 통합
async fn get_users(
    State(db): State<Arc<Mutex<SqliteDatabase>>>
) -> Result<Json<Vec<User>>, StatusCode> {
    let mut db = db.lock().await;
    let results = db.query("SELECT * FROM users").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // User 구조체로 변환
    Ok(Json(users))
}
```

### 다른 Rust 시스템과 통합

```rust
// rust_logger_system과 통합
let logger = Logger::new();
let mut db = SqliteDatabase::new();

match db.connect(":memory:").await {
    Ok(_) => logger.info("데이터베이스 연결됨"),
    Err(e) => logger.error(format!("데이터베이스 오류: {}", e)),
}
```

---

*아키텍처 문서 버전 1.0*
*최종 업데이트: 2025-10-16*
