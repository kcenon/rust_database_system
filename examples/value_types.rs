//! Value types example
//!
//! This example demonstrates working with different database value types:
//! - Type conversions
//! - Type safety
//! - Null handling
//! - Value extraction
//!
//! Run with: cargo run --example value_types

use rust_database_system::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Rust Database System - Value Types Example ===\n");

    let db = SqliteDatabase::new();
    db.connect(":memory:").await?;

    // Create a table with various data types
    println!("1. Creating table with various data types...");
    db.execute(
        "CREATE TABLE data_types (
            id INTEGER PRIMARY KEY,
            bool_val INTEGER,
            int_val INTEGER,
            long_val INTEGER,
            float_val REAL,
            double_val REAL,
            string_val TEXT,
            blob_val BLOB,
            null_val INTEGER
        )",
    )
    .await?;
    println!("   ✓ Table created\n");

    // Insert data with different types
    println!("2. Inserting data with different types...");
    db.execute(
        "INSERT INTO data_types VALUES (
            1,
            1,
            42,
            9223372036854775807,
            3.14,
            2.718281828459045,
            'Hello, World!',
            X'48656C6C6F',
            NULL
        )",
    )
    .await?;
    println!("   ✓ Data inserted\n");

    // Query and display values
    println!("3. Querying and displaying values...\n");
    let results = db.query("SELECT * FROM data_types").await?;

    if let Some(row) = results.first() {
        // Boolean value
        println!("   Boolean value:");
        if let Some(val) = row.get("bool_val") {
            println!("     Raw: {:?}", val);
            println!("     As bool: {:?}", val.as_bool());
            println!("     As int: {:?}", val.as_int());
            println!("     Type: {}", val.type_name());
        }
        println!();

        // Integer value
        println!("   Integer value:");
        if let Some(val) = row.get("int_val") {
            println!("     Raw: {:?}", val);
            println!("     As int: {:?}", val.as_int());
            println!("     As long: {:?}", val.as_long());
            println!("     As double: {:?}", val.as_double());
            println!("     As string: {}", val.as_string());
            println!("     Type: {}", val.type_name());
        }
        println!();

        // Long value
        println!("   Long value:");
        if let Some(val) = row.get("long_val") {
            println!("     Raw: {:?}", val);
            println!("     As long: {:?}", val.as_long());
            println!("     As string: {}", val.as_string());
            println!("     Type: {}", val.type_name());
        }
        println!();

        // Float value
        println!("   Float value:");
        if let Some(val) = row.get("float_val") {
            println!("     Raw: {:?}", val);
            println!("     As float: {:?}", val.as_float());
            println!("     As double: {:?}", val.as_double());
            println!("     As string: {}", val.as_string());
            println!("     Type: {}", val.type_name());
        }
        println!();

        // Double value
        println!("   Double value:");
        if let Some(val) = row.get("double_val") {
            println!("     Raw: {:?}", val);
            println!("     As double: {:?}", val.as_double());
            println!("     As string: {}", val.as_string());
            println!("     Type: {}", val.type_name());
        }
        println!();

        // String value
        println!("   String value:");
        if let Some(val) = row.get("string_val") {
            println!("     Raw: {:?}", val);
            println!("     As string: {}", val.as_string());
            println!("     As bytes: {:?}", val.as_bytes());
            println!("     Type: {}", val.type_name());
        }
        println!();

        // Blob value
        println!("   Blob value:");
        if let Some(val) = row.get("blob_val") {
            println!("     Raw: {:?}", val);
            println!("     As bytes: {:?}", val.as_bytes());
            println!("     As string: {}", val.as_string());
            println!("     Type: {}", val.type_name());
        }
        println!();

        // Null value
        println!("   Null value:");
        if let Some(val) = row.get("null_val") {
            println!("     Raw: {:?}", val);
            println!("     Is null: {}", val.is_null());
            println!("     As int: {:?}", val.as_int());
            println!("     As string: {}", val.as_string());
            println!("     Type: {}", val.type_name());
        }
        println!();
    }

    // Demonstrate type conversions
    println!("4. Type conversion examples...\n");

    // Create values from Rust types
    let values = vec![
        ("Boolean", DatabaseValue::from(true)),
        ("Integer", DatabaseValue::from(42)),
        ("Long", DatabaseValue::from(1234567890i64)),
        ("Float", DatabaseValue::from(std::f32::consts::PI)),
        ("Double", DatabaseValue::from(std::f64::consts::E)),
        ("String", DatabaseValue::from("Rust")),
        ("Bytes", DatabaseValue::from(vec![1u8, 2, 3, 4, 5])),
        ("Null", DatabaseValue::from(Option::<i32>::None)),
    ];

    for (name, value) in values {
        println!("   {} ({}):", name, value.type_name());
        println!("     Value: {:?}", value);
        println!("     As string: {}", value.as_string());
        println!();
    }

    // Demonstrate safe type extraction
    println!("5. Safe type extraction...\n");

    db.execute(
        "INSERT INTO data_types VALUES (
            2,
            0,
            100,
            999,
            1.5,
            2.5,
            '42',
            X'00',
            NULL
        )",
    )
    .await?;

    let results = db.query("SELECT * FROM data_types WHERE id = 2").await?;
    let row = &results[0];

    // Extract with type checking
    if let Some(val) = row.get("int_val") {
        match val.as_int() {
            Some(i) => println!("   Integer value: {}", i),
            None => println!("   Could not convert to integer"),
        }
    }

    if let Some(val) = row.get("string_val") {
        // String that looks like a number
        println!("   String value: {}", val.as_string());
        println!("   Parsed as int: {:?}", val.as_int());
        println!("   Parsed as long: {:?}", val.as_long());
    }

    if let Some(val) = row.get("null_val") {
        if val.is_null() {
            println!("   Null value detected, using default: 0");
        }
    }

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
