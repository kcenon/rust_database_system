//! Criterion benchmarks for rust_database_system

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_database_system::prelude::*;

// ============================================================================
// DatabaseValue Creation Benchmarks
// ============================================================================

fn bench_value_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_creation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("bool", |b| {
        b.iter(|| {
            let value = DatabaseValue::from(black_box(true));
            black_box(value)
        });
    });

    group.bench_function("int", |b| {
        b.iter(|| {
            let value = DatabaseValue::from(black_box(42i32));
            black_box(value)
        });
    });

    group.bench_function("long", |b| {
        b.iter(|| {
            let value = DatabaseValue::from(black_box(123456789i64));
            black_box(value)
        });
    });

    group.bench_function("float", |b| {
        b.iter(|| {
            let value = DatabaseValue::from(black_box(std::f32::consts::PI));
            black_box(value)
        });
    });

    group.bench_function("double", |b| {
        b.iter(|| {
            let value = DatabaseValue::from(black_box(std::f64::consts::PI));
            black_box(value)
        });
    });

    group.bench_function("string", |b| {
        b.iter(|| {
            let value = DatabaseValue::from(black_box("Hello, World!".to_string()));
            black_box(value)
        });
    });

    group.bench_function("bytes", |b| {
        let data = vec![1u8, 2, 3, 4, 5];
        b.iter(|| {
            let value = DatabaseValue::from(black_box(data.clone()));
            black_box(value)
        });
    });

    group.bench_function("null", |b| {
        b.iter(|| {
            let value = DatabaseValue::from(black_box(Option::<i32>::None));
            black_box(value)
        });
    });

    group.finish();
}

// ============================================================================
// Type Conversion Benchmarks
// ============================================================================

fn bench_type_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_conversions");
    group.throughput(Throughput::Elements(1));

    let int_val = DatabaseValue::from(42i32);
    let long_val = DatabaseValue::from(123456789i64);
    let double_val = DatabaseValue::from(std::f64::consts::PI);
    let string_val = DatabaseValue::from("Hello, World!".to_string());

    group.bench_function("int_to_long", |b| {
        b.iter(|| {
            let result = int_val.as_long();
            black_box(result)
        });
    });

    group.bench_function("int_to_double", |b| {
        b.iter(|| {
            let result = int_val.as_double();
            black_box(result)
        });
    });

    group.bench_function("long_to_double", |b| {
        b.iter(|| {
            let result = long_val.as_double();
            black_box(result)
        });
    });

    group.bench_function("double_to_string", |b| {
        b.iter(|| {
            let result = double_val.as_string();
            black_box(result)
        });
    });

    group.bench_function("string_clone", |b| {
        b.iter(|| {
            let result = string_val.as_string();
            black_box(result)
        });
    });

    group.finish();
}

// ============================================================================
// DatabaseRow Operations Benchmarks
// ============================================================================

fn bench_row_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_operations");

    // Benchmark row insertion with different sizes
    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("insert", size), size, |b, &size| {
            b.iter(|| {
                let mut row = DatabaseRow::new();
                for i in 0..size {
                    row.insert(format!("col_{}", i), DatabaseValue::from(i));
                }
                black_box(row)
            });
        });
    }

    // Benchmark row lookups
    let mut row = DatabaseRow::new();
    for i in 0..100 {
        row.insert(format!("col_{}", i), DatabaseValue::from(i));
    }

    group.bench_function("get_first", |b| {
        b.iter(|| {
            let value = row.get(black_box("col_0"));
            black_box(value)
        });
    });

    group.bench_function("get_middle", |b| {
        b.iter(|| {
            let value = row.get(black_box("col_50"));
            black_box(value)
        });
    });

    group.bench_function("get_last", |b| {
        b.iter(|| {
            let value = row.get(black_box("col_99"));
            black_box(value)
        });
    });

    group.bench_function("get_nonexistent", |b| {
        b.iter(|| {
            let value = row.get(black_box("nonexistent"));
            black_box(value)
        });
    });

    group.finish();
}

// ============================================================================
// Row Clone Benchmarks
// ============================================================================

fn bench_row_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_clone");

    for size in [10, 50, 100].iter() {
        let mut row = DatabaseRow::new();
        for i in 0..*size {
            row.insert(format!("col_{}", i), DatabaseValue::from(i));
        }

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &row, |b, row| {
            b.iter(|| {
                let cloned = row.clone();
                black_box(cloned)
            });
        });
    }

    group.finish();
}

// ============================================================================
// JSON Serialization Benchmarks
// ============================================================================

fn bench_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization");

    // Benchmark different value types
    let values = vec![
        ("bool", DatabaseValue::from(true)),
        ("int", DatabaseValue::from(42i32)),
        ("long", DatabaseValue::from(123456789i64)),
        ("double", DatabaseValue::from(std::f64::consts::PI)),
        ("string", DatabaseValue::from("Hello, World!".to_string())),
        ("null", DatabaseValue::from(Option::<i32>::None)),
    ];

    for (name, value) in values.iter() {
        group.bench_with_input(BenchmarkId::new("value", name), value, |b, value| {
            b.iter(|| {
                let json = serde_json::to_string(value).unwrap();
                black_box(json)
            });
        });
    }

    // Benchmark row serialization with different sizes
    for size in [10, 50, 100].iter() {
        let mut row = DatabaseRow::new();
        for i in 0..*size {
            row.insert(format!("col_{}", i), DatabaseValue::from(i));
            row.insert(
                format!("str_{}", i),
                DatabaseValue::from(format!("value_{}", i)),
            );
        }

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("row", size), &row, |b, row| {
            b.iter(|| {
                let json = serde_json::to_string(row).unwrap();
                black_box(json)
            });
        });
    }

    group.finish();
}

// ============================================================================
// Value Comparison Benchmarks
// ============================================================================

fn bench_value_comparisons(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_comparisons");
    group.throughput(Throughput::Elements(1));

    let int1 = DatabaseValue::from(42i32);
    let str1 = DatabaseValue::from("hello".to_string());

    group.bench_function("type_check_int", |b| {
        b.iter(|| {
            let is_int = int1.as_int().is_some();
            black_box(is_int)
        });
    });

    group.bench_function("type_check_string", |b| {
        b.iter(|| {
            let is_string = !str1.as_string().is_empty();
            black_box(is_string)
        });
    });

    group.bench_function("null_check", |b| {
        b.iter(|| {
            let is_null = int1.is_null();
            black_box(is_null)
        });
    });

    group.bench_function("type_name", |b| {
        b.iter(|| {
            let type_name = int1.type_name();
            black_box(type_name)
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_value_creation,
    bench_type_conversions,
    bench_row_operations,
    bench_row_clone,
    bench_json_serialization,
    bench_value_comparisons
);

criterion_main!(benches);
