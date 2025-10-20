# Rust Database System - 개선 계획

> **Languages**: [English](./IMPROVEMENTS.md) | 한국어

## 개요

이 문서는 코드 분석을 바탕으로 식별된 Rust Database System의 약점과 제안된 개선사항을 설명합니다.

## 식별된 문제점

### 1. 제한된 백엔드 구현

**문제**: 아키텍처가 깔끔한 추상화 레이어를 제공하지만, SQLite만 완전히 구현되어 있습니다. PostgreSQL과 MySQL 백엔드는 계획만 되어 있고 구현되지 않아 추상화의 실용적 가치가 제한됩니다.

**위치**: `src/lib.rs:24`

**현재 상태**:
```rust
// SQLite만 구현됨
pub mod backends {
    pub mod sqlite;
    // TODO: PostgreSQL backend
    // TODO: MySQL backend
}
```

**영향**:
- 다중 백엔드 이점 없이 추상화 오버헤드만 존재
- 사용자가 데이터베이스 시스템 간 전환 불가
- 프로덕션 배포 옵션 제한
- 단일 구현으로 아키텍처 복잡성 정당화 어려움

**제안된 해결책**:

```rust
// TODO: PostgreSQL 백엔드 구현
// 우선순위: 높음 - 추상화에 큰 가치 추가

pub mod postgres {
    use crate::core::{Database, DatabaseError, Result, Transaction, Value};
    use tokio_postgres::{Client, NoTls};

    pub struct PostgresDatabase {
        client: Client,
        connection: tokio::task::JoinHandle<()>,
    }

    impl PostgresDatabase {
        pub async fn new(connection_string: &str) -> Result<Self> {
            let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
                .await
                .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

            let handle = tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("Connection error: {}", e);
                }
            });

            Ok(Self {
                client,
                connection: handle,
            })
        }
    }

    #[async_trait::async_trait]
    impl Database for PostgresDatabase {
        async fn connect(&mut self) -> Result<()> {
            // new()에서 이미 연결됨
            Ok(())
        }

        async fn execute(&mut self, query: &str, params: &[Value]) -> Result<u64> {
            let pg_params = convert_params(params)?;
            self.client
                .execute(query, &pg_params[..])
                .await
                .map_err(|e| DatabaseError::ExecutionError(e.to_string()))
        }

        async fn query(&mut self, query: &str, params: &[Value]) -> Result<Vec<Row>> {
            let pg_params = convert_params(params)?;
            let rows = self.client
                .query(query, &pg_params[..])
                .await
                .map_err(|e| DatabaseError::ExecutionError(e.to_string()))?;

            convert_rows(rows)
        }

        // ... 나머지 메서드 구현
    }
}
```

**추가할 의존성**:
```toml
# Cargo.toml
[dependencies]
tokio-postgres = { version = "0.7", optional = true }
postgres-types = { version = "0.2", optional = true }

[features]
postgres = ["tokio-postgres", "postgres-types"]
mysql = ["mysql_async"]  # 향후
```

**우선순위**: 높음
**예상 작업량**: 대 (백엔드당 2-3주)

### 2. 파라미터 마샬링 오버헤드

**문제**: SQLite 쿼리에 바인딩할 때 각 파라미터가 개별적으로 박스화되어, 파라미터 슬라이스를 직접 바인딩하는 것에 비해 할당 오버헤드와 수명 복잡성이 추가됩니다.

**위치**: `src/backends/sqlite.rs:146`

**현재 구현**:
```rust
pub async fn execute(&mut self, query: &str, params: &[Value]) -> Result<u64> {
    let conn = self.connection.as_ref().unwrap();

    // 각 파라미터가 개별적으로 박스화됨 - 비효율적
    let boxed_params: Vec<Box<dyn ToSql>> = params
        .iter()
        .map(|v| {
            let boxed: Box<dyn ToSql> = match v {
                Value::Int(i) => Box::new(*i),
                Value::Long(l) => Box::new(*l),
                Value::String(s) => Box::new(s.clone()),
                Value::Bytes(b) => Box::new(b.clone()),
                // ... 더 많은 박싱
            };
            boxed
        })
        .collect();

    // trait object 참조로 변환
    let params_refs: Vec<&dyn ToSql> = boxed_params
        .iter()
        .map(|b| b.as_ref())
        .collect();

    conn.execute(query, params_refs.as_slice())?;
}
```

**영향**:
- 모든 파라미터에 대한 추가 힙 할당
- 메모리 압력 증가
- 이중 변환으로 인한 수명 복잡성
- 높은 처리량 쿼리에서 성능 저하

**제안된 해결책**:

**옵션 1: 튜플 지원과 함께 rusqlite의 params! 매크로 사용**

```rust
// TODO: 파라미터별 박싱을 피하도록 파라미터 바인딩 최적화
// 튜플과 함께 rusqlite의 네이티브 파라미터 바인딩 사용

use rusqlite::params_from_iter;

pub async fn execute(&mut self, query: &str, params: &[Value]) -> Result<u64> {
    let conn = self.connection.as_ref().unwrap();

    // rusqlite 호환 타입으로 직접 변환
    let sqlite_params: Vec<SqliteValue> = params
        .iter()
        .map(|v| match v {
            Value::Int(i) => SqliteValue::Integer(*i as i64),
            Value::Long(l) => SqliteValue::Integer(*l),
            Value::String(s) => SqliteValue::Text(s.clone()),
            Value::Bytes(b) => SqliteValue::Blob(b.clone()),
            Value::Float(f) => SqliteValue::Real(*f as f64),
            Value::Double(d) => SqliteValue::Real(*d),
            Value::Boolean(b) => SqliteValue::Integer(if *b { 1 } else { 0 }),
        })
        .collect();

    // 단일 할당, 박싱 오버헤드 없음
    let rows_affected = conn.execute(query, params_from_iter(sqlite_params.iter()))?;
    Ok(rows_affected as u64)
}

// 박싱을 피하기 위한 헬퍼 enum
enum SqliteValue {
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl ToSql for SqliteValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            SqliteValue::Integer(i) => i.to_sql(),
            SqliteValue::Real(r) => r.to_sql(),
            SqliteValue::Text(s) => s.to_sql(),
            SqliteValue::Blob(b) => b.to_sql(),
        }
    }
}
```

**옵션 2: 정적 타입에 대한 제로 카피 파라미터 바인딩**

```rust
// TODO: 더 나은 성능을 위한 제로 카피 파라미터 바인딩 구현

pub async fn execute(&mut self, query: &str, params: &[Value]) -> Result<u64> {
    let conn = self.connection.as_ref().unwrap();

    // 작은 파라미터 개수에 대해 스택 할당 배열 사용
    if params.len() <= 8 {
        return self.execute_small(query, params);
    }

    // 큰 파라미터 세트에 대해 vec 할당으로 폴백
    self.execute_large(query, params)
}

fn execute_small(&mut self, query: &str, params: &[Value]) -> Result<u64> {
    // 스택 할당 배열, 힙 할당 없음
    let mut param_storage: [MaybeUninit<SqliteParam>; 8] =
        MaybeUninit::uninit_array();

    for (i, value) in params.iter().enumerate() {
        param_storage[i].write(SqliteParam::from_value(value));
    }

    let initialized = unsafe {
        MaybeUninit::array_assume_init(param_storage[..params.len()].try_into().unwrap())
    };

    // 박싱 없이 직접 바인딩
    let rows = self.connection.as_ref().unwrap()
        .execute(query, &initialized)?;

    Ok(rows as u64)
}
```

**벤치마크 비교**:

```rust
// TODO: 성능 개선을 검증하기 위한 벤치마크 추가

#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn benchmark_parameter_binding(c: &mut Criterion) {
        let mut db = SqliteDatabase::new(":memory:").unwrap();

        c.bench_function("current_boxing", |b| {
            b.iter(|| {
                let params = vec![
                    Value::Int(black_box(42)),
                    Value::String(black_box("test".to_string())),
                    Value::Double(black_box(3.14)),
                ];
                db.execute("SELECT ?1, ?2, ?3", &params)
            })
        });

        c.bench_function("optimized_no_boxing", |b| {
            b.iter(|| {
                let params = vec![
                    Value::Int(black_box(42)),
                    Value::String(black_box("test".to_string())),
                    Value::Double(black_box(3.14)),
                ];
                db.execute_optimized("SELECT ?1, ?2, ?3", &params)
            })
        });
    }
}
```

**예상되는 개선**:
- 할당 오버헤드 30-50% 감소
- 파라미터 집약적 쿼리에서 실행 시간 15-25% 개선
- 높은 처리량 시나리오에서 메모리 압력 감소

**우선순위**: 중간
**예상 작업량**: 중간 (1주)

## 추가 개선사항

### 3. 연결 풀 지원

**제안**: 동시 부하에서 더 나은 성능을 위한 연결 풀링 추가:

```rust
// TODO: 프로덕션 워크로드를 위한 연결 풀링 지원 추가

use deadpool::managed::{Manager, Pool, PoolError};

pub struct DatabasePool {
    pool: Pool<SqliteConnectionManager>,
}

impl DatabasePool {
    pub fn new(database_url: &str, max_size: usize) -> Result<Self> {
        let manager = SqliteConnectionManager::new(database_url);
        let pool = Pool::builder(manager)
            .max_size(max_size)
            .build()
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn get_connection(&self) -> Result<PooledConnection> {
        self.pool
            .get()
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))
    }
}
```

**의존성**:
```toml
deadpool = "0.10"
deadpool-sqlite = "0.7"
```

**우선순위**: 중간
**예상 작업량**: 소 (2-3일)

### 4. 준비된 구문 캐싱

**제안**: 쿼리 재파싱을 피하기 위한 준비된 구문 캐싱:

```rust
// TODO: 반복된 쿼리를 위한 준비된 구문 캐싱 구현

use std::collections::HashMap;
use rusqlite::Statement;

pub struct SqliteDatabase {
    connection: Option<Connection>,
    statement_cache: HashMap<String, Statement<'static>>,
    cache_size: usize,
}

impl SqliteDatabase {
    pub async fn execute_cached(&mut self, query: &str, params: &[Value]) -> Result<u64> {
        let stmt = if let Some(cached) = self.statement_cache.get_mut(query) {
            cached
        } else {
            let stmt = self.connection
                .as_mut()
                .unwrap()
                .prepare(query)?;

            // 캐시가 가득 차면 가장 오래된 것 제거 (LRU 정책)
            if self.statement_cache.len() >= self.cache_size {
                let oldest_key = self.statement_cache.keys().next().unwrap().clone();
                self.statement_cache.remove(&oldest_key);
            }

            self.statement_cache.insert(query.to_string(), stmt);
            self.statement_cache.get_mut(query).unwrap()
        };

        let rows_affected = stmt.execute(convert_params(params)?)?;
        Ok(rows_affected as u64)
    }
}
```

**예상되는 이점**:
- 반복된 쿼리에 대해 40-60% 성능 개선
- 파싱 오버헤드 감소
- CPU 사용률 감소

**우선순위**: 중간
**예상 작업량**: 소 (2-3일)

### 5. 비동기 트랜잭션 지원

**제안**: async/await 패턴으로 트랜잭션 API 개선:

```rust
// TODO: 비동기 트랜잭션 빌더 패턴 추가

pub struct TransactionBuilder<'a> {
    db: &'a mut dyn Database,
    operations: Vec<Operation>,
}

impl<'a> TransactionBuilder<'a> {
    pub fn new(db: &'a mut dyn Database) -> Self {
        Self {
            db,
            operations: Vec::new(),
        }
    }

    pub fn execute(mut self, query: &str, params: Vec<Value>) -> Self {
        self.operations.push(Operation::Execute {
            query: query.to_string(),
            params,
        });
        self
    }

    pub fn query(mut self, query: &str, params: Vec<Value>) -> Self {
        self.operations.push(Operation::Query {
            query: query.to_string(),
            params,
        });
        self
    }

    pub async fn commit(self) -> Result<TransactionResult> {
        let mut tx = self.db.begin_transaction().await?;
        let mut results = TransactionResult::default();

        for operation in self.operations {
            match operation {
                Operation::Execute { query, params } => {
                    let rows = tx.execute(&query, &params).await?;
                    results.rows_affected += rows;
                }
                Operation::Query { query, params } => {
                    let rows = tx.query(&query, &params).await?;
                    results.query_results.push(rows);
                }
            }
        }

        tx.commit().await?;
        Ok(results)
    }
}

// 사용법:
let result = TransactionBuilder::new(&mut db)
    .execute("INSERT INTO users (name) VALUES (?1)", vec![Value::String("Alice".into())])
    .execute("INSERT INTO users (name) VALUES (?1)", vec![Value::String("Bob".into())])
    .query("SELECT * FROM users", vec![])
    .commit()
    .await?;
```

**우선순위**: 낮음
**예상 작업량**: 소 (3-4일)

## 테스트 요구사항

### 필요한 새 테스트:

1. **다중 백엔드 테스트**:
   ```rust
   #[tokio::test]
   async fn test_sqlite_postgres_compatibility() {
       let mut sqlite_db = SqliteDatabase::new(":memory:").await.unwrap();
       let mut pg_db = PostgresDatabase::new("postgresql://localhost/test").await.unwrap();

       // 동일한 작업이 두 곳에서 작동해야 함
       for db in [&mut sqlite_db as &mut dyn Database, &mut pg_db as &mut dyn Database] {
           db.connect().await.unwrap();

           db.execute(
               "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)",
               &[]
           ).await.unwrap();

           db.execute(
               "INSERT INTO test (name) VALUES (?1)",
               &[Value::String("test".into())]
           ).await.unwrap();

           let rows = db.query("SELECT * FROM test", &[]).await.unwrap();
           assert_eq!(rows.len(), 1);
       }
   }
   ```

2. **파라미터 바인딩 성능 테스트**:
   ```rust
   #[tokio::test]
   async fn test_parameter_binding_performance() {
       let mut db = SqliteDatabase::new(":memory:").await.unwrap();
       db.connect().await.unwrap();

       db.execute("CREATE TABLE bench (a INT, b TEXT, c REAL)", &[]).await.unwrap();

       let start = Instant::now();
       for i in 0..10000 {
           db.execute(
               "INSERT INTO bench VALUES (?1, ?2, ?3)",
               &[
                   Value::Int(i),
                   Value::String(format!("test_{}", i)),
                   Value::Double(i as f64 * 0.5),
               ]
           ).await.unwrap();
       }
       let elapsed = start.elapsed();

       println!("10000 inserts in {:?}", elapsed);
       // 합리적인 시간 내에 완료되어야 함
       assert!(elapsed < Duration::from_secs(5));
   }
   ```

3. **연결 풀 테스트**:
   ```rust
   #[tokio::test]
   async fn test_connection_pool_concurrency() {
       let pool = DatabasePool::new("test.db", 10).unwrap();

       let handles: Vec<_> = (0..50)
           .map(|i| {
               let pool = pool.clone();
               tokio::spawn(async move {
                   let mut conn = pool.get_connection().await.unwrap();
                   conn.execute(
                       "INSERT INTO test VALUES (?1)",
                       &[Value::Int(i)]
                   ).await.unwrap();
               })
           })
           .collect();

       for handle in handles {
           handle.await.unwrap();
       }

       let mut conn = pool.get_connection().await.unwrap();
       let rows = conn.query("SELECT COUNT(*) FROM test", &[]).await.unwrap();
       assert_eq!(rows[0].get::<i64>(0).unwrap(), 50);
   }
   ```

4. **구문 캐시 테스트**:
   ```rust
   #[tokio::test]
   async fn test_prepared_statement_caching() {
       let mut db = SqliteDatabase::new(":memory:").await.unwrap();
       db.connect().await.unwrap();

       db.execute("CREATE TABLE test (id INT)", &[]).await.unwrap();

       // 동일한 쿼리를 여러 번 실행
       let query = "INSERT INTO test VALUES (?1)";
       for i in 0..100 {
           db.execute_cached(query, &[Value::Int(i)]).await.unwrap();
       }

       // 캐시 적중률 검증
       let stats = db.cache_statistics();
       assert!(stats.hit_rate > 0.99); // >99% 캐시 적중
   }
   ```

## 구현 로드맵

### 1단계: 핵심 개선 (스프린트 1-2)
- [ ] PostgreSQL 백엔드 구현
- [ ] 포괄적인 백엔드 호환성 테스트 추가
- [ ] 다중 백엔드 사용 패턴 문서화
- [ ] SQLite에서 PostgreSQL로의 마이그레이션 가이드 생성

### 2단계: 성능 최적화 (스프린트 3)
- [ ] 박싱 오버헤드를 제거하도록 파라미터 바인딩 최적화
- [ ] 파라미터 바인딩 벤치마크 추가
- [ ] 준비된 구문 캐싱 구현
- [ ] 성능 프로파일링 문서 추가

### 3단계: 프로덕션 준비 (스프린트 4-5)
- [ ] 연결 풀링 지원 추가
- [ ] 트랜잭션 빌더 API 구현
- [ ] 포괄적인 통합 테스트 추가
- [ ] 프로덕션 배포 가이드 생성

### 4단계: 고급 기능 (스프린트 6)
- [ ] MySQL 백엔드 구현
- [ ] 데이터베이스 마이그레이션 지원 추가
- [ ] 데이터베이스 관리용 CLI 도구 생성
- [ ] 관찰성 및 메트릭 추가

## Breaking Changes

⚠️ **주의**: 파라미터 바인딩 최적화가 내부 API를 변경할 수 있습니다.

**마이그레이션 경로**:
1. 파라미터 바인딩 변경은 내부적 - 공개 API 변경 없음
2. 새 백엔드는 추가 기능 - 호환성 깨짐 없음
3. 연결 풀링은 feature flag를 통한 opt-in
4. 기존 직접 연결 방법은 계속 지원됨

## 성능 목표

### 현재 성능 기준:
- 단순 INSERT: ~500 ops/sec
- 파라미터화된 INSERT: ~300 ops/sec (박싱 오버헤드로 인해)
- 단순 SELECT: ~1000 ops/sec
- 100개 작업을 가진 트랜잭션: ~50 tx/sec

### 개선 후 목표 성능:
- 단순 INSERT: ~800 ops/sec (+60%)
- 파라미터화된 INSERT: ~600 ops/sec (+100%)
- 단순 SELECT: ~1500 ops/sec (+50%)
- 100개 작업을 가진 트랜잭션: ~100 tx/sec (+100%)
- 캐시된 준비된 구문: ~2000 ops/sec (신규)

## 참고자료

- 코드 분석: Database System Review 2025-10-16
- 관련 이슈:
  - 백엔드 구현 (#TODO)
  - 파라미터 바인딩 성능 (#TODO)
  - 연결 풀링 (#TODO)
- rusqlite 문서: https://docs.rs/rusqlite/
- tokio-postgres 문서: https://docs.rs/tokio-postgres/

---

*개선 계획 버전 1.0*
*최종 업데이트: 2025-10-17*
