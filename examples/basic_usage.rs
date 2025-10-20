//! Basic database usage example
//!
//! This example demonstrates basic database operations including:
//! - Connecting to a database
//! - Creating tables
//! - Inserting data
//! - Querying data
//! - Working with different value types
//!
//! Run with: cargo run --example basic_usage

use rust_database_system::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Rust Database System - Basic Usage Example ===\n");

    // Create a new SQLite database (in-memory)
    let db = SqliteDatabase::new();

    // Connect to database
    println!("1. Connecting to database...");
    db.connect(":memory:").await?;
    println!("   ✓ Connected\n");

    // Create table
    println!("2. Creating table...");
    db.execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            email TEXT NOT NULL,
            age INTEGER,
            balance REAL,
            is_active INTEGER DEFAULT 1
        )",
    )
    .await?;
    println!("   ✓ Table created\n");

    // Insert data using parameterized queries (prevents SQL injection)
    println!("3. Inserting data...");
    let users = vec![
        ("alice", "alice@example.com", 30, 1500.50),
        ("bob", "bob@example.com", 25, 2300.75),
        ("charlie", "charlie@example.com", 35, 980.25),
        ("diana", "diana@example.com", 28, 3200.00),
    ];

    for (username, email, age, balance) in users {
        let affected = db
            .execute_with_params(
                "INSERT INTO users (username, email, age, balance) VALUES (?, ?, ?, ?)",
                &[
                    DatabaseValue::from(username),
                    DatabaseValue::from(email),
                    DatabaseValue::from(age),
                    DatabaseValue::from(balance),
                ],
            )
            .await?;
        println!("   ✓ Inserted {} row(s)", affected);
    }
    println!();

    // Query all users
    println!("4. Querying all users...");
    let results = db.query("SELECT * FROM users ORDER BY id").await?;
    println!("   Found {} users:", results.len());

    for row in &results {
        let id = row
            .get("id")
            .and_then(|v| v.as_long())
            .ok_or_else(|| DatabaseError::ColumnNotFound("id".to_string()))?;
        let username = row
            .get("username")
            .ok_or_else(|| DatabaseError::ColumnNotFound("username".to_string()))?
            .as_string();
        let email = row
            .get("email")
            .ok_or_else(|| DatabaseError::ColumnNotFound("email".to_string()))?
            .as_string();
        let age = row.get("age").and_then(|v| v.as_int()).unwrap_or(0);
        let balance = row
            .get("balance")
            .and_then(|v| v.as_double())
            .unwrap_or(0.0);

        println!(
            "   - User #{}: {} ({}) - Age: {}, Balance: ${:.2}",
            id, username, email, age, balance
        );
    }
    println!();

    // Query with filter
    println!("5. Querying users with balance > $1000...");
    let results = db
        .query("SELECT * FROM users WHERE balance > 1000.0 ORDER BY balance DESC")
        .await?;
    println!("   Found {} users:", results.len());

    for row in &results {
        let username = row
            .get("username")
            .ok_or_else(|| DatabaseError::ColumnNotFound("username".to_string()))?
            .as_string();
        let balance = row
            .get("balance")
            .and_then(|v| v.as_double())
            .ok_or_else(|| DatabaseError::ColumnNotFound("balance".to_string()))?;
        println!("   - {}: ${:.2}", username, balance);
    }
    println!();

    // Update data
    println!("6. Updating data...");
    let affected = db
        .execute("UPDATE users SET balance = balance + 100.0 WHERE age < 30")
        .await?;
    println!("   ✓ Updated {} row(s)\n", affected);

    // Query updated data
    println!("7. Querying updated balances...");
    let results = db
        .query("SELECT username, balance FROM users WHERE age < 30")
        .await?;

    for row in &results {
        let username = row
            .get("username")
            .ok_or_else(|| DatabaseError::ColumnNotFound("username".to_string()))?
            .as_string();
        let balance = row
            .get("balance")
            .and_then(|v| v.as_double())
            .ok_or_else(|| DatabaseError::ColumnNotFound("balance".to_string()))?;
        println!("   - {}: ${:.2} (bonus applied)", username, balance);
    }
    println!();

    // Delete data
    println!("8. Deleting inactive users...");
    db.execute("UPDATE users SET is_active = 0 WHERE balance < 1000.0")
        .await?;
    let affected = db.execute("DELETE FROM users WHERE is_active = 0").await?;
    println!("   ✓ Deleted {} inactive user(s)\n", affected);

    // Final count
    println!("9. Final user count...");
    let results = db.query("SELECT COUNT(*) as count FROM users").await?;
    let count = results
        .first()
        .and_then(|row| row.get("count"))
        .and_then(|v| v.as_long())
        .ok_or_else(|| DatabaseError::QueryError("Failed to get count".to_string()))?;
    println!("   Remaining users: {}\n", count);

    // Disconnect
    println!("10. Disconnecting...");
    db.disconnect().await?;
    println!("    ✓ Disconnected");

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
